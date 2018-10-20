extern crate encoding;
extern crate regex;
extern crate chrono;

// db 
#[macro_use]
extern crate diesel;
extern crate dotenv;

// toml
#[macro_use]
extern crate serde_derive;
extern crate toml;

//http
extern crate iron;
extern crate router;
extern crate mount;
extern crate staticfile;
extern crate handlebars_iron as hbs;
extern crate params;

mod mesi;
mod webs;
mod sqlib;

use std::io::prelude::*;
use std::io::{BufReader, LineWriter};
use std::error::Error;
use std::net::TcpStream;

use std::string::String;

use std::fs::OpenOptions;
use std::path::Path;

use encoding::{Encoding, DecoderTrap, EncoderTrap};
use encoding::all::ISO_2022_JP;

use regex::Regex;
use chrono::{Local};

use std::sync::{Arc, Mutex};
use std::thread;
use std::sync::mpsc::{channel, Sender, Receiver};

use std::collections::VecDeque;

use mesi::Mesi;

//type BWriter<'a> = LineWriter<&'a TcpStream>;
type Mesg = Arc<Mutex<VecDeque<String>>>;

#[derive(Deserialize)]
pub struct Setting {
    irc:Irc,
    log:Log,
    webs:Webs,
}

#[derive(Deserialize)]
struct Irc {
    server:String,
    password:String,
    nick:String,
    channel:String,
}

#[derive(Deserialize)]
struct Log {
    dir:String,
}

#[derive(Deserialize)]
struct Webs{
    host:String,
    username:String,
    password:String,
}

fn main() {
    println!("Hello, world!");
    let filename = "robot.toml";
    let mut setting_string = String::new();
    OpenOptions::new().read(true).open(filename).unwrap()
        .read_to_string(&mut setting_string).unwrap();

    let setting:Setting = toml::from_str(&setting_string).unwrap();

    let (send, recv):(Sender<String>, Receiver<String>) = channel();
    let messages:Mesg = Arc::new(Mutex::new(VecDeque::with_capacity(32)));

    webs::run_webs(&setting, send.clone(), messages.clone());
    connect_irc(setting, send, recv, messages).unwrap();
}

fn connect_irc(setting:Setting, send:Sender<String>, recv:Receiver<String>, messages:Mesg) -> Result<(), Box<Error>>{

    let conn = Arc::new(TcpStream::connect(&setting.irc.server).unwrap());
    let mut bstream = BufReader::new(&(*conn));

    let logdir = setting.log.dir.clone();
    let do_message = move |line: &str| {
        let filename = Path::new(&logdir).join(Local::now().format("irc%Y%m%d.txt").to_string());
        let out = format!("{}{}\n",Local::now().format("%H:%M:%S"), line);
        OpenOptions::new().create(true).append(true).open(filename).unwrap()
            .write(out.as_bytes()).unwrap();
        
        messages.lock().unwrap().truncate(10);
        messages.lock().unwrap().push_front(out);
    };

    let re_num = Regex::new(r"\d{3}").unwrap();
    let send_irc = Mutex::new(send.clone());
    let do_irc_fn = move |line:&String, mesi:Option<&mut Mesi>|{
        let sp: Vec<&str> = line.split(" ").collect();
        if sp.len() < 2 {
            return
        }
        let (from, command, to, opt) = {
            let (from, start) = if sp[0].starts_with(":") {(sp[0].get(1..).unwrap(), 1)} else {("", 0)};
            let fromsp: Vec<&str> = from.split("!").collect();
            (fromsp[0], sp[start], sp[start+1], 
                if sp.len() > start+2 {sp[(start+2)..].join(" ")} else {String::from("")}
            )
        };
        match command {
            "PING" => {
                send_irc.lock().unwrap().send(
                    format!("PONG {}\r\n", to)
                ).unwrap();
            },
            "PONG" => {},
            "PRIVMSG" =>{
                match mesi {
                    Some(mesi) =>{
                        if opt.starts_with(":mesi") {
                            mesi.receive(from, to, &opt);
                        }
                    }
                    None => {}
                }
                do_message(&format!("<{}>{}", from, opt));
            },
            "NOTICE" => {
                do_message(&format!("={}={}", from, opt));
            },
            "433" => {
                // nick name already used
                panic!("nick name alredy used");
            },
            x if re_num.is_match(x) =>{
                // info message
                //println!("{}", line);
            },
            _ => {
                do_message(&line);
            }
        }
    };
    let do_irc = Arc::new(do_irc_fn);

    let acon = conn.clone();
    let th_irc = do_irc.clone();
    thread::spawn(move || {
        let mut stream = LineWriter::new(&(*acon));
        loop{
            let s = recv.recv().unwrap();
            th_irc(&s, None);
            let en = ISO_2022_JP.encode(s.as_str(), EncoderTrap::Ignore)
                .unwrap_or(b"".to_vec());
            stream.write(en.as_slice()).unwrap();
            stream.write(b"\r\n").unwrap();
        }
    });

    send.send(format!("PASS {}", setting.irc.password)).unwrap();
    send.send(format!("NICK {}", setting.irc.nick)).unwrap();
    send.send(format!("USER {} 0 * :mesi by rust", setting.irc.nick)).unwrap();
    send.send(format!("JOIN {}", setting.irc.channel)).unwrap();

    let mut mesi = Mesi::new(send.clone());

    println!("connect irc");
    loop {
        let mut bl = String::new();
        bstream.read_line(&mut bl).unwrap();

        let line = ISO_2022_JP.decode(&bl.as_bytes()[..bl.len()-2], DecoderTrap::Ignore)
            .unwrap_or("".to_string());
        //println!("{}",line);
        do_irc(&line, Some(&mut mesi));
    }
}
