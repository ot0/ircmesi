use regex::Regex;

use sqlib;
use super::send_command;
use super::BWriter;

pub struct Mesi {
    now_project:usize
}

impl Mesi {
    pub fn new() -> Self{
        Mesi{
            now_project:0
        }
    }
    pub fn recieve(&mut self, stream: &mut BWriter, from: &str, to:&str, opt:&String) {
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
            if start == 2 {
                self.now_project = parties.len()-1;
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
