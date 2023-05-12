use std::str;
use std::process::Command;

struct Event {
    uid: u32,
    name: String,
    description: String,
    date: String,
    time: String,
    file: String,
}

struct Action {
    uid: u32,
    name: String,
    description: String,
    date: String,
    time: String,
    file: String,
}

impl Event {
    fn execute(&self) {
        println!("Event: {}", self.name);
    }

}

// replace with a database
struct EventStore {
    events: Vec<Event>,
    actions: Vec<Action>,
}

fn main() {
    println!("Hello, world!");

    let event: Event = Event {
        uid: 1,
        name: "test".to_string(),
        description: "test".to_string(),
        date: "test".to_string(),
        time: "test".to_string(),
        file: "".to_string(),
    };

    let action: Action = Action {
        uid: 1,
        name: "test".to_string(),
        description: "test".to_string(),
        date: "test".to_string(),
        time: "test".to_string(),
        file: "test".to_string(),
    };


    let mut event_store = EventStore {
        events : Vec::new(),
        actions: Vec::new()
    };

    event_store.events.push(event);
    event_store.actions.push(action);

    engine(event_store);

}

fn action_executor(action: Action) {
    println!("Action Executor");
    let output = Command::new("ls")
        .arg("a")
        .arg(action.file)
        .output()
        .expect("failed to execute process");

    println!("status: {}", output.status);
    println!("stdout: {}", str::from_utf8(&output.stdout).unwrap());
    println!("stderr: {}", str::from_utf8(&output.stderr).unwrap());
}


fn executor(file_path: String) {
    println!("executing  file: {}", file_path);
    let output = Command::new("ls")
        .arg("a")
        .arg(file_path)
        .output()
        .expect("failed to execute process");

    println!("status: {}", output.status);
    println!("stdout: {}", str::from_utf8(&output.stdout).unwrap());
    println!("stderr: {}", str::from_utf8(&output.stderr).unwrap());
}


fn engine(event_store: EventStore) {
    println!("Engine");

    loop {
        for event in event_store.events {
            println!("Event: {}", event.name);
    
            if executor(event) != null {
                println!("Successfully executed event: {}", event.name);
            }
        }
        // sleep for 1 second
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
