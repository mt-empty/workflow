use anyhow::{Error as AnyError, Ok, Result};
use serde_derive::Deserialize;
use serde_derive::Serialize;
use std::fs::File;

use crate::utils::insert_event_into_db;
use crate::utils::push_tasks_to_queue;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Workflow {
    pub name: Option<String>,
    pub description: Option<String>,
    pub events: Vec<Event>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
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

pub fn process(yaml_file_path: String) -> Result<(), AnyError> {
    let workflow = parse_yaml_file(&yaml_file_path)?;
    println!("name: {:?}", workflow.name);
    println!("description: {:?}", workflow.description);

    // let mut events = Vec::new();

    for e in workflow.events {
        // println!("name: {:?}", e.name);
        // println!("description: {:?}", e.description);
        // println!("trigger: {:?}", e.trigger);
        let mut tasks = Vec::new();
        for a in e.tasks {
            // println!("name: {:?}", a.name);
            // println!("description: {:?}", a.description);
            // println!("path: {:?}", a.path);
            // println!("path: {:?}", a.on_failure);

            let task = Task {
                name: a.name,
                description: a.description,
                path: a.path,
                on_failure: a.on_failure,
            };

            tasks.push(task.clone());
        }

        let event = Event {
            name: e.name,
            description: e.description,
            trigger: e.trigger,
            tasks: tasks.clone(),
        };
        insert_event_into_db(event)?;
        // events.push(event.clone());
    }

    // println!("events: {:?}", events);

    Ok(())
}
