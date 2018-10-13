extern crate encoding;
extern crate regex;
extern crate chrono;
#[macro_use]
extern crate diesel;
extern crate dotenv;

#[macro_use]
extern crate serde_derive;
extern crate toml;

use std::io::prelude::*;
use std::error::Error;
use std::net::TcpStream;

use std::string::String;

use std::fs::OpenOptions;
use std::path::Path;

use encoding::{Encoding, DecoderTrap, EncoderTrap};
use encoding::all::ISO_2022_JP;

use regex::Regex;
use chrono::{Local};

mod sqlib;

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

    connect_irc().unwrap();
}

fn connect_irc() -> Result<(), Box<Error>>{
    let filename = "robot.toml";
    let mut setting_string = String::new();
    OpenOptions::new().read(true).open(filename)?
        .read_to_string(&mut setting_string)?;

    let setting:Setting = toml::from_str(&setting_string)?;

    let mut stream = TcpStream::connect(&setting.irc.server)?;
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
        let ret = ISO_2022_JP.decode(&buf[0..size], DecoderTrap::Ignore)
            .unwrap_or("".to_string());
        //println!("f:");
        for line in ret.split("\r\n") {
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

fn send_command(stream: &mut TcpStream, s:String){
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

struct Mesi {
    now_project:usize
}

impl Mesi {
    pub fn new() -> Self{
        Mesi{
            now_project:0
        }
    }
    pub fn recieve(&mut self, stream: &mut TcpStream, from: &str, to:&str, opt:&String) {
        let conn = sqlib::establish_connection();
        let parties = sqlib::get_party(&conn);

        let sp: Vec<&str> = opt.split(" ").collect();
        if sp.len() < 2 {
            return;
        }

        let re_num = Regex::new(r"\d+").unwrap();
        let (target, start) = if re_num.is_match(sp[1])
            {(sp[1].parse().unwrap(), 2)}
            else {(self.now_project, 1)};

        let pid = if parties.len() ==0 {
            -1
        }else if target >= parties.len() {
            if start ==2 {
                self.now_project = 0;
            }
            -1
        }else{
            self.now_project = target;
            parties[target].id
        };
        
        let command = sp[start];
        let opt = if sp.len() <= start+1{
            from.to_string()
        }else { 
            sp[(start+1)..].join(" ").trim().to_string()
        };

        match command {
            "shows" => {
                for (i, pt) in parties.iter().enumerate() {
                    send_command(stream, 
                        format!("NOTICE {} :{}, {}, {}", to, i, pt.title, pt.create_time)
                    );
                    let mut names:Vec<String> = Vec::new();
                    for mem in sqlib::get_member(&conn, pt.id){
                        names.push(mem.name);
                    }
                    send_command(stream,
                        format!("NOTICE {} :{}", to, names.join(","))
                    );
                }                
            },
            "+" =>{
                let mem = sqlib::get_member_id(&conn, &opt, pid);
                if mem.len() == 0 {
                    sqlib::add_member(&conn, &opt, pid);
                }else{
                    println!("already {} to {}", from, pid);
                }
            },
            "-" =>{
                let mem = sqlib::get_member_id(&conn, &opt, pid);
                if mem.len() != 0 {
                    sqlib::del_member(&conn, mem[0].id);
                }else{
                    println!("no member {} to {}", from, pid);
                }
            },
            "add" =>{
                sqlib::add_party(&conn, &opt);
                self.now_project = parties.len();
            },
            "del" =>{
                sqlib::enable_party(&conn, pid, false);
            }
            _ => {
                println!("{},{},{},{}", command, from, opt, to);
            }
        }
    }
}
