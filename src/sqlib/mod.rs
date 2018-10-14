pub mod schema;
pub mod models;

use dotenv::dotenv;
use diesel;
use diesel::prelude::*;
//use diesel::types::Timestamp;
use std::env;
use std::vec::Vec;
use self::models::*;


pub fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
}

pub fn add_party(conn: &SqliteConnection, title: &str) -> usize {
    use self::schema::party;

    let np = NewParty {
        title: title,
    };

    diesel::insert_into(party::table)
        .values(&np)
        .execute(conn)
        .expect("Error saving new post")
}

pub fn add_member(conn: &SqliteConnection, name: &str, pid:i32) -> usize {
    use self::schema::member;

    let nm = NewMember {
        name: name,
        pid: pid,
    };

    diesel::insert_into(member::table)
        .values(&nm)
        .execute(conn)
        .expect("Error saving new post")
}

pub fn enable_party(conn: &SqliteConnection, pid:i32, enable:bool){
    use self::schema::party::dsl::{party, id, valid};
    diesel::update(party.filter(id.eq(pid)))
        .set(valid.eq(enable))
        .execute(conn).unwrap();
}

pub fn get_party(conn: &SqliteConnection) -> Vec<Party>{
    use self::schema::party::dsl::{party, valid, create_time};
    party.filter(valid.eq(true))
        .order(create_time)
        .load::<Party>(conn)
        .expect("Error load")
}

pub fn get_all_party(conn: &SqliteConnection) -> Vec<Party>{
    use self::schema::party::dsl::{party, valid, create_time};
    party
        .order((valid, create_time.desc()))
        .load::<Party>(conn)
        .expect("Error load")
}

pub fn get_member(conn: &SqliteConnection, did:i32) -> Vec<Member>{
    use self::schema::member::dsl::{member, pid};
    member.filter(pid.eq(did))
        .load::<Member>(conn)
        .expect("Error Member")
}

pub fn get_member_id(conn: &SqliteConnection, dname:&str, did:i32) -> Vec<Member>{
    use self::schema::member::dsl::{member, pid, name};
    member.filter(pid.eq(did).and(name.eq(dname)))
        .load::<Member>(conn)
        .expect("Error Member")
}

pub fn del_member(conn: &SqliteConnection, did:i32){
    use self::schema::member::dsl::{member, id};
    diesel::delete(member.filter(id.eq(did)))
        .execute(conn).unwrap();
}
