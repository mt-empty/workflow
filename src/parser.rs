use crate::models::NewEvent;
use crate::utils::{establish_pg_connection, insert_event_into_db, insert_event_tasks_into_db};
use anyhow::{Error as AnyError, Ok, Result};
use serde_derive::{Deserialize, Serialize};
use std::env;
use std::fs::File;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Workflow {
    pub name: Option<String>,
    pub description: Option<String>,
    pub events: Vec<ParsableEvent>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsableEvent {
    pub name: Option<String>,
    pub description: Option<String>,
    pub trigger: String,
    pub tasks: Vec<Task>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub name: Option<String>,
    pub description: Option<String>,
    pub path: String,
    pub on_failure: Option<String>,
}

fn parse_yaml_file(file_path: &str) -> Result<Workflow, AnyError> {
    let file = File::open(file_path)?;
    let workflow: Workflow = serde_yaml::from_reader(file)?;
    println!("{:?}", workflow);
    Ok(workflow)
}

pub fn process_yaml_file(yaml_file_path: String) -> Result<(), AnyError> {
    let workflow = parse_yaml_file(&yaml_file_path)?;
    println!("name: {:?}", workflow.name);
    println!("description: {:?}", workflow.description);

    let workflow_root_path = std::path::Path::new(&yaml_file_path)
        .parent()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();

    let workflow_path = env::current_dir()
        .expect("Failed to get current directory")
        .join(workflow_root_path);

    for e in workflow.events {
        let mut tasks = Vec::new();
        for t in e.tasks {
            let task = Task {
                name: t.name,
                description: t.description,
                path: workflow_path.join(t.path).to_str().unwrap().to_string(),
                on_failure: t.on_failure,
            };

            tasks.push(task.clone());
        }

        let new_event = ParsableEvent {
            name: e.name,
            description: e.description,
            trigger: workflow_path.join(e.trigger).to_str().unwrap().to_string(),
            tasks: tasks.clone(),
        };
        let mut conn = establish_pg_connection();
        let event_uid = insert_event_into_db(&mut conn, new_event)?;

        insert_event_tasks_into_db(&mut conn, tasks, event_uid)?;
    }

    Ok(())
}
