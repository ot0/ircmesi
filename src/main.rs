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

use mesi::Mesi;

type BWriter<'a> = LineWriter<&'a TcpStream>;

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
    port:u32,
}

fn main() {
    println!("Hello, world!");
    let filename = "robot.toml";
    let mut setting_string = String::new();
    OpenOptions::new().read(true).open(filename).unwrap()
        .read_to_string(&mut setting_string).unwrap();

    let setting:Setting = toml::from_str(&setting_string).unwrap();

    webs::run_webs(&setting);
    connect_irc(setting).unwrap();
}

fn connect_irc(setting:Setting) -> Result<(), Box<Error>>{

    let rstream = TcpStream::connect(&setting.irc.server)?;
    let mut bstream = BufReader::new(&rstream);
    let mut stream = LineWriter::new(&rstream);

    send_command(&mut stream, format!("PASS {}", setting.irc.password));
    send_command(&mut stream, format!("NICK {}", setting.irc.nick));
    send_command(&mut stream, format!("USER {} 0 * :mesi by rust", setting.irc.nick));
    send_command(&mut stream, format!("JOIN {}", setting.irc.channel));    

    let mut mesi = Mesi::new();
    let re_num = Regex::new(r"\d{3}").unwrap();

    println!("connect irc");
    loop {
        let mut bl = String::new();
        bstream.read_line(&mut bl).unwrap();

        let line = ISO_2022_JP.decode(&bl.as_bytes()[..bl.len()-2], DecoderTrap::Ignore)
            .unwrap_or("".to_string());
        //println!("{}",line);

        let sp: Vec<&str> = line.split(" ").collect();
        if sp.len() < 2 {
            continue
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
                //println!("Pong {}\n", to);
                send_command(&mut stream, format!("PONG {}\r\n", to));
            },
            "PRIVMSG" =>{
                if opt.starts_with(":mesi") {
                    mesi.recieve(&mut stream, from, to, &opt);
                }
                //println!("{} {} {}", command, from, opt);
                do_message(&format!("<{}>{}", from, opt), &setting);
            },
            "NOTICE" => {
                do_message(&format!("<{}>{}", from, opt), &setting);
            },
            "433" => {
                // nick name already used
                panic!("nick name alredy used");
            },
            x if re_num.is_match(x) =>{
                //println!("{}", line);
            },
            _ => {
                //println!("{} {} {}", command, from, opt);
                do_message(&line, &setting);
            }
        }
    }
}

pub fn send_command(stream:&mut BWriter, s:String){
    let en = ISO_2022_JP.encode(s.as_str(), EncoderTrap::Ignore)
        .unwrap_or(b"".to_vec());
    stream.write(en.as_slice()).unwrap();
    stream.write(b"\r\n").unwrap();
}

fn do_message(line: &str, setting: &Setting) {
    let filename = Path::new(&setting.log.dir).join(Local::now().format("irc%Y%m%d.txt").to_string());
    OpenOptions::new().create(true).append(true).open(filename).unwrap()
        .write(format!("{}{}\n",Local::now().format("%H:%M:%S"), line).as_bytes()).unwrap();
    //println!("{}", line);
}
