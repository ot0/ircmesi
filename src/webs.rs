
use std::collections::HashMap;
use std::vec::Vec;
use std::error::Error;
//use std::time::Duration;

use iron::prelude::*;
use iron::{headers, middleware, status};
use iron::typemap::TypeMap;
use iron::headers::ContentType;
use iron::Handler;

use hyper_native_tls::NativeTlsServer;

use router::Router;
use mount::Mount;
use hbs::{Template, HandlebarsEngine, DirectorySource};
use staticfile::Static;

use std::path::Path;
use std::fs;
use std::fmt;
//use std::process::Command;

use std::thread;
use std::sync::mpsc::Sender;
use std::sync::Mutex;

use regex::Regex;

use super::Setting;
use sqlib::{establish_connection, get_all_party, get_member};
use super::Mesg;

fn top_handler(_req: &mut Request, log_dir:&str) -> IronResult<Response> {
    let mut resp = Response::new();
    let mut data = HashMap::new();

    let conn = establish_connection();
    let mut mesi_list:Vec<HashMap<String, String>> = Vec::new();
    for pt in get_all_party(&conn) {
        let mut party = HashMap::new();
        party.insert("id".to_string(), format!("{}",pt.id));
        party.insert("title".to_string(), pt.title);
        party.insert("create".to_string(), format!("{}", pt.create_time));
        party.insert("enable".to_string(), 
            if pt.valid {"○".to_string()} else{"×".to_string()});
        
        let mut names:Vec<String> = Vec::new();
        for mem in get_member(&conn, pt.id){
            names.push(mem.name);
        }
        party.insert("member".to_string(), names.join(", "));
        party.insert("number".to_string(), format!("{}", names.len()));
        mesi_list.push(party);
    }
    data.insert("mesi_list".to_string(), mesi_list);

    let mut log_list:Vec<HashMap<String, String>> = Vec::new();
    let mut filelist:Vec<String> = fs::read_dir(log_dir).unwrap()
        .map(|r| r.unwrap().file_name().into_string().unwrap_or("code error".to_string()))
        .collect();
    filelist.sort_unstable();
    filelist.reverse();
    for filename in filelist {
        let mut logf = HashMap::new();
        //let filename = log.unwrap().file_name().into_string()
        //    .unwrap_or("code error".to_string());
        logf.insert("dir".to_string(), format!("log/{}", filename));
        logf.insert("name".to_string(), filename);
        log_list.push(logf);
    }
    data.insert("log_list".to_string(), log_list);
    resp.set_mut(Template::new("index", data)).set_mut(status::Ok);
    return Ok(resp);
}

fn hello_page(_: &mut Request) -> IronResult<Response> {
    Ok(Response::with((status::Ok, "Hello world")))
}

pub fn run_webs(setting:&Setting, send:Sender<String>, messages:Mesg){
    let msend = Mutex::new(send);
    let log_dir = setting.log.dir.to_string();

    //Create Router
    let mut router = Router::new();
    let pagename = setting.irc.channel.clone();
    let nick = setting.irc.nick.clone();
    let urlre = Regex::new("((https?|ftp)(://[-_.!~*'()a-zA-Z0-9;/?:@&=+$,%#]+))").unwrap();
    router
        .get("/", move |req: &mut Request|-> IronResult<Response> {
            top_handler(req, &log_dir)
        }, "index")
        .get("/dice", move |_:&mut Request| -> IronResult<Response> {
            msend.lock().unwrap().send(format!(":{} PRIVMSG {} :2D6",nick, pagename)).unwrap();
            Ok(Response::with((status::Ok, "dice")))
        }, "dice")
        .get("/msg", move |_:&mut Request| -> IronResult<Response>{
            let mut msg = "".to_string();
            for line in (*messages.lock().unwrap()).iter(){
                msg += &format!("<div>{}</div>", line
                    .replace("&","&amp;")
                    .replace("<","&lt;")
                    .replace(">","&gt;")
                    //.replace(" ", "&nbsp;")
                );
            }
            let msg = urlre.replace_all(&msg, "<a href=\"$0\">$0</a>");
            Ok(Response::with((status::Ok, format!("{}", msg))))
        }, "msg")
        .get("/hello", hello_page, "hello");

    let mut mount = Mount::new();
    let sld = Static::new(Path::new(&format!("{}/", setting.log.dir)));
    mount
        .mount("/", router)
        .mount("/resources", Static::new(Path::new("resources/")))
        .mount("/log", move |req: &mut Request| -> IronResult<Response> {
            match sld.handle(req) {
                Ok(mut res)=>{
                    res.headers.set(ContentType::plaintext());
                    Ok(res)
                }
                other => other
            }
        });

    //Create Chain
    let mut chain = Chain::new(mount);
    
    let bam = BasicAuthMiddleware::new(&setting.webs.username, &setting.webs.password);
    chain.link_before(bam);

    // Add HandlerbarsEngine to middleware Chain
    let mut hbse = HandlebarsEngine::new();
    hbse.add(Box::new(
        DirectorySource::new("./templates/", ".hbs")));
    if let Err(r) = hbse.reload() {
        panic!("{}", r.description());
    }
    chain.link_after(hbse);
    
    //let host = format!("localhost:{}",setting.webs.port);
    let host = setting.webs.host.clone();
    let pem = setting.webs.pem.clone();
    if pem != "" {
        println!("connect: https://{}", host);
        let ssl = NativeTlsServer::new(pem, "").unwrap();
        thread::spawn(move || {
            Iron::new(chain).https(host, ssl).unwrap();
        });
    }else{
        println!("connect: http://{}", host);
        thread::spawn(move || {
            Iron::new(chain).http(host).unwrap();
        });
    }
}

#[derive(Debug)]
struct AuthError;

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt("authentication error", f)
    }
}

impl Error for AuthError {
    fn description(&self) -> &str {
        "authentication error"
    }
}


struct BasicAuthMiddleware {
    name:String,
    pass:String,
}


impl BasicAuthMiddleware{
    fn new(name:&String, pass:&String) -> Self{
        BasicAuthMiddleware{
            name:name.clone(),
            pass:pass.clone(),
        }
    }

    fn response_auth(&self) -> IronResult<()>{
        let mut hs = headers::Headers::new();
        hs.set_raw("WWW-Authenticate", vec![b"Basic realm=\"main\"".to_vec()]);
        Err(IronError {
            error: Box::new(AuthError),
            response: Response {
                status: Some(status::Unauthorized),
                headers: hs,
                extensions: TypeMap::new(),
                body: None
            }
        })
    }
}

impl middleware::BeforeMiddleware for BasicAuthMiddleware {
    fn before(&self, req: &mut Request) -> IronResult<()> {
        match req.headers.get::<headers::Authorization<headers::Basic>>() {
            Some(&headers::Authorization(headers::Basic { ref username, password: Some(ref password) })) => {
                if *username == self.name && *password == self.pass {
                    Ok(())
                } else {
                    self.response_auth()
                }
            },
            _ => {
                self.response_auth()
            }
        }
    }
}