use std::str;
use std::process::Command;



struct Event {
    uid: u32,
    name: str,
    description: str,
    date: str,
    time: str,
}

struct Action {
    uid: u32,
    name: str,
    description: str,
    date: str,
    time: str,
}

// replace with a database
struct EventStore {
    events: Vec<Event>,
    actions: Vec<Action>,
}

fn main() {
    println!("Hello, world!");

    let mut event_store = EventStore {
        events : Vec::new(),
        actions: Vec::new()
    };


}

fn engine() {
    println!("Engine");


}