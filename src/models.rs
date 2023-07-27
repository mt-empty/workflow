// use crate::schema::sql_types::EngineStatus;
use diesel::deserialize::{self, FromSql};
use diesel::pg::{Pg, PgValue};
use diesel::prelude::*;
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::Bool;
use diesel::*;
use serde_derive::{Deserialize, Serialize};
use std::io::Write;

use crate::engine::EventStatus;

// #[derive(SqlType)]
// #[diesel(postgres_type(name = "My_Type"))]
// pub struct MyType;

// #[derive(Debug, PartialEq, FromSqlRow, AsExpression, Eq)]
// #[diesel(sql_type = MyType)]
// pub enum MyEnum {
//     Running,
//     Stopped,
// }

// pub enum EngineStatus {
// }

// impl ToSql<MyType, Pg> for MyEnum {
//     fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
//         match *self {
//             MyEnum::Foo => out.write_all(b"foo")?,
//             MyEnum::Bar => out.write_all(b"bar")?,
//         }
//         Ok(IsNull::No)
//     }
// }

// impl FromSql<MyType, Pg> for MyEnum {
//     fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
//         match bytes.as_bytes() {
//             b"foo" => Ok(MyEnum::Foo),
//             b"bar" => Ok(MyEnum::Bar),
//             _ => Err("Unrecognized enum variant".into()),
//         }
//     }
// }
#[derive(Queryable, Selectable, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::engines)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Engine {
    pub uid: i32,
    pub name: String,
    pub ip_address: String,
    pub status: String, // TODO: change to enum
    pub stop_signal: bool,
    pub started_at: chrono::NaiveDateTime,
    pub stopped_at: chrono::NaiveDateTime,
}

#[derive(Insertable, Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::engines)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewEngine<'a> {
    pub name: &'a str,
    pub ip_address: &'a str,
}

#[derive(Queryable, Selectable, Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::events)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Event {
    pub uid: i32,
    pub name: Option<String>,
    pub description: Option<String>,
    pub trigger: String,
    pub status: String, // TODO: change to enum
    pub created_at: chrono::NaiveDateTime,
    pub triggered_at: Option<chrono::NaiveDateTime>,
    pub deleted_at: Option<chrono::NaiveDateTime>,
}

#[derive(Insertable, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::events)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewEvent<'a> {
    pub name: Option<&'a str>,
    pub description: Option<&'a str>,
    pub trigger: &'a str,
    pub status: String,
    pub created_at: chrono::NaiveDateTime,
}

impl Default for NewEvent<'_> {
    fn default() -> Self {
        NewEvent {
            name: None,
            description: None,
            trigger: "",
            status: EventStatus::Created.to_string(),
            created_at: chrono::Local::now().naive_local(),
        }
    }
}

#[derive(Queryable, Selectable, Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::tasks)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Task {
    pub uid: i32,
    pub event_uid: i32,
    pub name: Option<String>,
    pub description: Option<String>,
    pub path: String,
    pub on_failure: Option<String>,
    pub status: String, // TODO: change to enum
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub deleted_at: Option<chrono::NaiveDateTime>,
    pub completed_at: Option<chrono::NaiveDateTime>,
}

#[derive(Insertable, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::tasks)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewTask {
    pub event_uid: i32,
    pub name: Option<String>,
    pub description: Option<String>,
    pub path: String,
    pub on_failure: Option<String>,
    pub status: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

impl Default for NewTask {
    fn default() -> Self {
        NewTask {
            event_uid: 0,
            name: None,
            description: None,
            path: "".to_string(),
            on_failure: None,
            status: EventStatus::Created.to_string(),
            created_at: chrono::Local::now().naive_local(),
            updated_at: chrono::Local::now().naive_local(),
        }
    }
}
