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
use std::time::Duration;
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
    let mrecv = Arc::new(Mutex::new(recv));

    webs::run_webs(&setting, msend.clone(), messages.clone());

    loop{
        let err = connect_irc(&setting, msend.clone(), mrecv.clone(), messages.clone());
        match err {
            Err(e) => {
                println!("err: {:?}", e);
            }
            _ => {
                println!("connection close");
            }
        }
        //msend.lock().unwrap().send("threadEnd".to_string()).unwrap();
        thread::sleep(Duration::new(10, 0));
    }
}

fn connect_irc(setting:&Setting, send:MSend, recv:Arc<Mutex<Receiver<String>>>, messages:Mesg) -> Result<(), Box<dyn Error>>
{
    let logdir = setting.log.dir.clone();
    let msg = messages.clone();
    let do_message = move |line: &str| {
        let filename = Path::new(&logdir).join(Local::now().format("irc%Y%m%d.txt").to_string());
        let out = format!("{}{}\n",Local::now().format("%H:%M:%S"), line);
        OpenOptions::new().create(true).append(true).open(filename).unwrap()
            .write(out.as_bytes()).unwrap();
        
        msg.lock().unwrap().append(out);
        println!("{}", line);
    };

    let send_irc = send.clone();
    let nick = setting.irc.nick.clone();
    let channel = setting.irc.channel.clone();
    let msg = messages.clone();
    let mesi_enable = setting.irc.mesi.clone();

    let do_irc_fn = Arc::new(move |line:&String, mesi:&Arc<Mutex<Mesi>>|{
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
            "PONG" => {}
            "PRIVMSG" =>{
                if mesi_enable && (opt.starts_with(":mesi") || opt.starts_with(":meshi")){
                    mesi.lock().unwrap().receive(from, to, &opt);
                }
                do_message(&format!("<{}>{}", from, opt));
            },
            "NOTICE" => {
                do_message(&format!("={}={}", from, opt));
            },
            "332" | "TOPIC" => {
                // topic
                do_message(&format!("<{}>/topic {}", from, opt));
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
                println!("{}", line)
            }
        }
    });
        
    let conn = Arc::new(TcpStream::connect(&setting.irc.server)?);
    println!("connect irc");
    
    let mmesi = Arc::new(Mutex::new(Mesi::new(send.clone())));
    //let do_irc = Arc::new(do_irc_fn);

    let acon = conn.clone();
    let do_irc = do_irc_fn.clone();
    let mesi = mmesi.clone();
    thread::spawn(move || {
    let mut bstream = BufReader::new(&(*acon));
        loop {
            let mut bl = String::new();
            match bstream.read_line(&mut bl) {
                Err(x) =>{
                    println!("recv close:{:?}", x);
                    break;
                },
                _ =>{}
            };
            if bl.len() == 0 {
                println!("recv len 0");
                break;
            }
            let line = ISO_2022_JP.decode(&bl.as_bytes()[..bl.len()-2], DecoderTrap::Ignore)
                .unwrap_or("".to_string());
            //println!("{}",line);
            &do_irc(&line, &mesi);
        }
    });

    send.lock().unwrap().send(format!("PASS {}", setting.irc.password))?;
    send.lock().unwrap().send(format!("NICK {}", setting.irc.nick))?;
    send.lock().unwrap().send(format!("USER {} 0 * :mesi by rust", setting.irc.nick))?;
    send.lock().unwrap().send(format!(":{} JOIN {}", setting.irc.nick, setting.irc.channel))?;

    let mut stream = LineWriter::new(&(*conn));
    let mesi = mmesi.clone();
    let do_irc = do_irc_fn.clone();
    loop{
        let s = recv.lock().unwrap().recv().unwrap();

        let en = ISO_2022_JP.encode(s.as_str(), EncoderTrap::Ignore)
            .unwrap_or(b"".to_vec());
        let err = stream.write(en.as_slice());
        match err {
            Err(x) => {
                println!("recv steam closed:{:?}", x);
                break;
            },
            _ => {}
        }
        stream.write(b"\r\n").unwrap();
        if s.starts_with("QUIT"){
            break;
        }
        &do_irc(&s, &mesi);
    }
    Ok(())
}

