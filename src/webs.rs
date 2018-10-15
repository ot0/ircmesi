
use std::collections::HashMap;
use std::vec::Vec;
use std::error::Error;
//use std::time::Duration;

use iron::prelude::*;
use iron::{headers, middleware, status};
use iron::typemap::TypeMap;

use router::Router;
use mount::Mount;
use hbs::{Template, HandlebarsEngine, DirectorySource};
use staticfile::Static;

use std::path::Path;
use std::fs;
use std::fmt;
//use std::process::Command;

use std::thread;

use super::Setting;
use sqlib::{establish_connection, get_all_party, get_member};

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
    for log in fs::read_dir(log_dir).unwrap() {
        let mut logf = HashMap::new();
        let filename = log.unwrap().file_name().into_string()
            .unwrap_or("code error".to_string());
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

pub fn run_webs(setting:&Setting){

    let log_dir = setting.log.dir.to_string();

    //Create Router
    let mut router = Router::new();
    router
        .get("/", move |req: &mut Request|-> IronResult<Response> {
            top_handler(req, &log_dir)
        }, "index")
        .get("/hello", hello_page, "hello");

    let mut mount = Mount::new();
    mount
        .mount("/", router)
        .mount("/resources", Static::new(Path::new("resources/")))
        .mount("/log", Static::new(
            Path::new(&format!("{}/", setting.log.dir))));

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
    println!("connect: http://{}", host);

    thread::spawn(move || {
        Iron::new(chain).http(host).unwrap();
    });
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