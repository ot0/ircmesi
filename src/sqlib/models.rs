use super::schema::{party, member};
//use diesel::sql_types::{Nullable, Timestamp};
use chrono::NaiveDateTime;

#[derive(Queryable,  PartialEq)]
pub struct Party {
    pub id: i32,
    pub title: String,
    //pub open_time: Nullable<Timestamp>,
    pub open_time: Option<NaiveDateTime>,
    pub create_time: NaiveDateTime,
    //pub create_time: Timestamp,
    pub valid: bool
}

#[derive(Queryable)]
pub struct Member {
    pub id: i32,
    pub name: String,
    pub pid: i32,
    pub create_time: NaiveDateTime
}

#[derive(Insertable)]
#[table_name = "party"]
pub struct NewParty<'a> {
    pub title: &'a str,
}

#[derive(Insertable)]
#[table_name = "member"]
pub struct NewMember<'a> {
    pub name: &'a str,
    pub pid: i32,
}

