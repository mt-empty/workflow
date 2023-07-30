// use crate::schema::sql_types::EngineStatus;
use diesel::prelude::*;
use serde_derive::{Deserialize, Serialize};

use crate::engine::EventStatus;

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
