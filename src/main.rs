extern crate encoding;
extern crate regex;
extern crate chrono;

#[macro_use]
extern crate serde_derive;
extern crate toml;

use std::io::prelude::*;
use std::net::TcpStream;

use std::string::String;

use std::fs::OpenOptions;
use std::path::Path;

use encoding::{Encoding, DecoderTrap, EncoderTrap};
use encoding::all::ISO_2022_JP;

use regex::Regex;
use chrono::{Local};

#[derive(Deserialize)]
struct Setting {
    irc:Irc,
    log:Log,
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

fn main() {
    println!("Hello, world!");

    connect_irc();
}

fn connect_irc(){
    let filename = "robot.toml";
    let mut setting_string = String::new();
    OpenOptions::new().read(true).open(filename).unwrap()
        .read_to_string(&mut setting_string).unwrap();

    let setting:Setting = toml::from_str(&setting_string).unwrap();

    let mut stream = TcpStream::connect(&setting.irc.server).unwrap();
    let mut buf = [0; 4096];

    send_command(&mut stream, format!("PASS {}", setting.irc.password));
    send_command(&mut stream, format!("NICK {}", setting.irc.nick));
    send_command(&mut stream, format!("USER {} 0 * :mesi by rust", setting.irc.nick));
    send_command(&mut stream, format!("JOIN {}", setting.irc.channel));    

    let mut mesi = Mesi::new();
    let re_num = Regex::new(r"\d{3}").unwrap();

    loop {
        let size = stream.read(&mut buf).unwrap();
        //let moto = std::str::from_utf8).unwrap();
        let ret = ISO_2022_JP.decode(&buf[0..size], DecoderTrap::Ignore).unwrap();
        //println!("f:");
        for line in ret.split("\r\n") {
            let sp: Vec<&str> = line.split(" ").collect();
            if sp.len() < 2 {
                continue
            }
            let (from, command, to, opt) = {
                let (from, start) = if sp[0].starts_with(":") {(sp[0], 1)} else {("", 0)};
                let fromsp: Vec<&str> = line.split("!").collect();
                (fromsp[0], sp[start], sp[start+1], 
                    if sp.len() > start+2 {sp[(start+2)..].join(" ")} else {String::from("")}
                )
            };
            //println!("{}",line);
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
                x if re_num.is_match(x) =>{
                    println!("{}", line);
                },
                _ => {
                    //println!("{} {} {}", command, from, opt);
                    do_message(line, &setting);
                }
            }
        }
    }
}

fn send_command(stream: &mut TcpStream, s:String) {
    ISO_2022_JP.encode(s.as_str(), EncoderTrap::Ignore).unwrap();
    stream.write(s.as_bytes()).unwrap();
    stream.write(b"\r\n").unwrap();
}

fn do_message(line: &str, setting: &Setting) {
    let filename = Path::new(&setting.log.dir).join(Local::now().format("irc%Y%m%d.txt").to_string());
    let mut file = OpenOptions::new().create(true).append(true).open(filename).unwrap();
    file.write(format!("{}{}\n",Local::now().format("%H:%M:%S"), line).as_bytes()).unwrap();
    println!("{}", line);
}

struct Mesi {
    now_project:i32
}

impl Mesi {
    pub fn new() -> Self{
        Mesi{
            now_project:0
        }
    }
    pub fn recieve(&self, stream: &mut TcpStream, from: &str, to:&str, opt:&String) {
        println!("{},{}", from, opt);
        //send_command(stream, format!("NOTICE {} {}", from, opt));
    }
}
