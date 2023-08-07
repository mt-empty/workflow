use std::fmt::{self, Display, Formatter};

use diesel::prelude::*;
use serde_derive::{Deserialize, Serialize};

// TODO: change status to enum

#[derive(Queryable, Selectable, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::engines)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Engine {
    pub uid: i32,
    pub name: String,
    pub ip_address: String,
    pub status: String,
    pub stop_signal: bool,
    pub started_at: chrono::NaiveDateTime,
    pub stopped_at: chrono::NaiveDateTime,
    pub task_process_status: String,
    pub event_process_status: String,
}

#[derive(Insertable, Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::engines)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewEngine<'a> {
    pub name: &'a str,
    pub ip_address: &'a str,
}

pub enum EngineStatus {
    Starting,
    Running,
    Stopped,
}

impl Display for EngineStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            EngineStatus::Starting => write!(f, "Starting"),
            EngineStatus::Running => write!(f, "Running"),
            EngineStatus::Stopped => write!(f, "Stopped"),
        }
    }
}

pub enum ProcessStatus {
    Running,
    Stopped,
}

impl Display for ProcessStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ProcessStatus::Running => write!(f, "Running"),
            ProcessStatus::Stopped => write!(f, "Stopped"),
        }
    }
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

pub enum EventStatus {
    Created,
    Succeeded,
    Retrying,
}

impl Display for EventStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            EventStatus::Created => write!(f, "Created"),
            EventStatus::Succeeded => write!(f, "Succeeded"),
            EventStatus::Retrying => write!(f, "Retrying"),
        }
    }
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::events)]
pub struct LightEvent {
    pub uid: i32,
    pub name: Option<String>,
    pub description: Option<String>,
    pub trigger: String,
    pub status: String,
}

impl fmt::Display for LightEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "\tuid: {}", self.uid)?;
        writeln!(f, "\tname: {:?}", self.name)?;
        writeln!(f, "\tdescription: {:?}", self.description)?;
        writeln!(f, "\ttrigger: {}", self.trigger)?;
        Ok(())
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

pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

impl Display for TaskStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "Pending"),
            TaskStatus::Running => write!(f, "Running"),
            TaskStatus::Completed => write!(f, "Completed"),
            TaskStatus::Failed => write!(f, "Failed"),
        }
    }
}

#[derive(Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::tasks)]
pub struct LightTask {
    pub uid: i32,
    pub path: String,
    pub on_failure: Option<String>,
}

impl Display for LightTask {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "\tuid: {}", self.uid)?;
        writeln!(f, "\tpath: {}", self.path)?;
        writeln!(
            f,
            "\ton_failure: {}",
            self.on_failure.as_ref().unwrap_or(&"None".to_string())
        )?;
        Ok(())
    }
}
