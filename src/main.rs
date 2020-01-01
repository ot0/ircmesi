extern crate encoding;
extern crate regex;
extern crate chrono;
extern crate getopts;

// db 
#[macro_use]
extern crate diesel;
extern crate dotenv;

// toml
#[macro_use]
extern crate serde_derive;
extern crate toml;
#[macro_use]
extern crate serde_json;

//http
extern crate iron;
extern crate mount;
extern crate staticfile;
extern crate handlebars_iron as hbs;
extern crate params;
extern crate hyper_native_tls;

extern crate grep_regex;
extern crate grep_printer;
extern crate grep_searcher;

mod mesi;
mod webs;
mod sqlib;

use std::io::prelude::*;
use std::io::{BufReader, LineWriter};
use std::error::Error;
use std::net::TcpStream;

use std::env;
use getopts::Options;

use std::string::String;

use std::fs::OpenOptions;
use std::path::Path;

use encoding::{Encoding, DecoderTrap, EncoderTrap};
use encoding::all::ISO_2022_JP;

//use regex::Regex;
use chrono::{Local};

use std::sync::{Arc, Mutex};
use std::thread;
use std::sync::mpsc::{channel, Sender, Receiver};

use std::collections::VecDeque;

use mesi::Mesi;

//type BWriter<'a> = LineWriter<&'a TcpStream>;
#[derive(Serialize, Debug)]
pub struct IrcMesg  {
    topic:String,
    member:String,
    cap:usize,
    log:VecDeque<String>,
    id:u64,
}
type Mesg = Arc<Mutex<IrcMesg>>;

impl IrcMesg {
    pub fn new(size:usize) -> Mesg {
        let msg = IrcMesg {
            topic:"".to_string(),
            member:"".to_string(),
            cap: size,
            log:VecDeque::with_capacity(size),
            id:0,
        };
        Arc::new(Mutex::new(msg))
    }
    pub fn append(&mut self, msg:String)-> &mut Self{
        self.log.truncate(self.cap-1);
        self.log.push_front(msg);
        self.id += 1;
        self
    }
}

#[derive(Deserialize, Debug)]
pub struct Setting {
    irc:Irc,
    log:Log,
    webs:Webs,
}

#[derive(Deserialize, Debug)]
struct Irc {
    server:String,
    password:String,
    nick:String,
    channel:String,
    mesi: bool,
    logsize: usize, 
}

#[derive(Deserialize, Debug)]
struct Log {
    dir:String,
}

#[derive(Deserialize, Debug)]
struct Webs{
    host:String,
    username:String,
    password:String,
    pem:String,
}

type MSend = Arc<Mutex<Sender<String>>>;

fn main() {
    println!("Hello, world!");
    
    let args: Vec<String> = env::args().collect();

    let mut opts = Options::new();
    opts.optopt("s", "", "set setting file name", "robot.toml");
    opts.optflag("h", "help", "print this help menu");

    let matches = opts.parse(&args[1..])
        .unwrap_or_else(|f| panic!(f.to_string()));

    if matches.opt_present("h") {
        print!("{}", opts.usage("Usage: mesi [options]"));
        return
    }

    let filename = match matches.opt_str("s") {
        Some(x) => x,
        None => "robot.toml".to_string()
    };

    let mut setting_string = String::new();
    OpenOptions::new().read(true).open(&filename).unwrap()
        .read_to_string(&mut setting_string).unwrap();

    let setting:Setting = toml::from_str(&setting_string).unwrap();

    let (send, recv):(Sender<String>, Receiver<String>) = channel();
    let messages:Mesg = IrcMesg::new(setting.irc.logsize);

    let msend = Arc::new(Mutex::new(send));

    webs::run_webs(&setting, msend.clone(), messages.clone());
    connect_irc(setting, msend, recv, messages).unwrap();
}

fn connect_irc(setting:Setting, send:MSend, recv:Receiver<String>, messages:Mesg) -> Result<(), Box<Error>>{

    let conn = Arc::new(TcpStream::connect(&setting.irc.server).unwrap());
    let mut bstream = BufReader::new(&(*conn));

    let logdir = setting.log.dir.clone();
    let msg = messages.clone();
    let do_message = move |line: &str| {
        let filename = Path::new(&logdir).join(Local::now().format("irc%Y%m%d.txt").to_string());
        let out = format!("{}{}\n",Local::now().format("%H:%M:%S"), line);
        OpenOptions::new().create(true).append(true).open(filename).unwrap()
            .write(out.as_bytes()).unwrap();
        
        msg.lock().unwrap().append(out);
    };

    let send_irc = send.clone();
    let nick = setting.irc.nick.clone();
    let channel = setting.irc.channel.clone();
    let msg = messages.clone();
    let do_irc_fn = move |line:&String, mesi:&mut Option<Mesi>|{
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
            "332" | "TOPIC" => {
                // topic
                do_message(&format!("/topic {}", opt));
                msg.lock().unwrap().topic = opt;
            }
            "353" => {
                // NAMES return
                //do_message(&format!("/member {}", opt));
                msg.lock().unwrap().member = opt;
            }
            "433" => {
                // nick name already used
                panic!("nick name alredy used");
            },
            "JOIN" | "QUIT" | "PART" => {
                if from != nick{
                    send_irc.lock().unwrap().send(format!("NAMES {}", channel)).unwrap();
                }
                do_message(&line);
            },
            
             "ERROR" => {
               do_message(&line);
            },
            _ => {
                //do_message(&line);
                //println!("{}", line)
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
            th_irc(&s, &mut None);
            let en = ISO_2022_JP.encode(s.as_str(), EncoderTrap::Ignore)
                .unwrap_or(b"".to_vec());
            stream.write(en.as_slice()).unwrap();
            stream.write(b"\r\n").unwrap();
        }
    });

    //let mut mesi =  Mesi::new(send.clone());
    let mut mesi = if setting.irc.mesi {
        Some(Mesi::new(send.clone()))
    }else{
        None
    };

    send.lock().unwrap().send(format!("PASS {}", setting.irc.password)).unwrap();
    send.lock().unwrap().send(format!("NICK {}", setting.irc.nick)).unwrap();
    send.lock().unwrap().send(format!("USER {} 0 * :mesi by rust", setting.irc.nick)).unwrap();
    send.lock().unwrap().send(format!(":{} JOIN {}", setting.irc.nick, setting.irc.channel)).unwrap();

    println!("connect irc");
    loop {
        let mut bl = String::new();
        bstream.read_line(&mut bl).unwrap();

        let line = ISO_2022_JP.decode(&bl.as_bytes()[..bl.len()-2], DecoderTrap::Ignore)
            .unwrap_or("".to_string());
        //println!("{}",line);
        do_irc(&line, &mut mesi);
    }
}
