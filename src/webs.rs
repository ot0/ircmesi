
use std::collections::HashMap;
use std::vec::Vec;
use std::error::Error;
//use std::time::Duration;
use std::path::Path;
use std::fs;
use std::fmt;
//use std::process::Command;

use std::thread;
use std::sync::mpsc::Sender;
use std::sync::Mutex;
use std::time::Instant;

use iron::prelude::*;
use iron::{headers, middleware, status};
use iron::typemap::TypeMap;
use iron::headers::ContentType;
use iron::Handler;

use hyper_native_tls::NativeTlsServer;

use mount::Mount;
use hbs::{Template, HandlebarsEngine, DirectorySource};
use staticfile::Static;

use params::{Params, Value};

use super::Setting;
use sqlib::{establish_connection, get_all_party, get_member};
use super::Mesg;

use grep_regex::RegexMatcher;
use grep_printer;
use grep_searcher::Searcher;

fn get_filelist(dir:&str) ->Vec<String> {
    let mut filelist:Vec<String>= fs::read_dir(dir).unwrap()
        .map(|r| r.unwrap().file_name().into_string().unwrap_or("code error".to_string()))
        .collect();
    filelist.sort_unstable();
    filelist.reverse();
    filelist
}

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
    let filelist = get_filelist(log_dir); 

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
fn grep(dir:&String, query:&String) ->Result<String, Box<Error>> {
    let mut printer = grep_printer::JSON::new(vec![]);
    let matcher = RegexMatcher::new(&*query)?;
    let mut searcher = Searcher::new();
    let filelist = get_filelist(dir);
    for filename in filelist {
        let path = format!("{}/{}",dir, filename);
        searcher.search_path(&matcher, path, printer.sink_with_path(&matcher, &filename))?;
    }
    let result = String::from_utf8(printer.into_inner())?;
    
    //println!("query:{}, {:?}", query, result);
    Ok(result.replace("\n",","))
}

pub fn run_webs(setting:&Setting, send:Sender<String>, messages:Mesg){

    let msend = Mutex::new(send);
    let log_dir = setting.log.dir.clone();
    let search_dir = setting.log.dir.clone();
    let pagename = setting.irc.channel.clone();
    let nick = setting.irc.nick.clone();
    let sld = Static::new(Path::new(&format!("{}/", setting.log.dir)));
    let before = Mutex::new(Instant::now());

    let mut mount = Mount::new();
    mount
        .mount("/", move |req: &mut Request|-> IronResult<Response> {
            top_handler(req, &log_dir)
        })
        .mount("/resources", Static::new(Path::new("resources/")))
        .mount("/log", move |req: &mut Request| -> IronResult<Response> {
            match sld.handle(req) {
                Ok(mut res)=>{
                    res.headers.set(ContentType::plaintext());
                    Ok(res)
                }
                other => other
            }
        })
        .mount("/msg", move |req:&mut Request| -> IronResult<Response>{
            let irc = messages.lock().unwrap();
            let map = req.get_ref::<Params>().unwrap();
            match map.find(&["id"]){
                Some(&Value::String(ref id)) if *id == irc.id.to_string() => {
                    Ok(Response::with(status::NoContent))
                }
                _ => {
                    Ok(Response::with((status::Ok, json!(*irc).to_string())))
                }
            }
        })
        .mount("/grep", move |req:&mut Request| -> IronResult<Response>{
            let map = req.get_ref::<Params>().unwrap();
            match map.find(&["q"]){
                Some(&Value::String(ref query)) =>{
                    //println!("{}", query);
                    match grep(&search_dir, query) {
                        Ok(msg) =>{
                            Ok(Response::with((status::Ok,format!("[{}{{\"type\":\"\"}}]",msg))))
                        } 
                        Err(err)=> {
                            println!("search error:{:?}", err);
                            Ok(Response::with((status::BadRequest, format!("{:?}", err))))
                        }
                    }
                }
                _ => {
                    Ok(Response::with((status::BadRequest, "query error")))
                }
            }
        })
        .mount("/dice", move |_:&mut Request| -> IronResult<Response> {
            let mut t = before.lock().unwrap();
            if t.elapsed().as_secs() > 10 {
                msend.lock().unwrap().send(format!(":{} PRIVMSG {} :2D6",nick, pagename)).unwrap();
                *t = Instant::now();
                Ok(Response::with((status::Ok, "dice")))
            }else{
                Ok(Response::with((status::ServiceUnavailable, "wait")))
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