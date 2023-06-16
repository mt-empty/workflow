use dotenv::dotenv;
use postgres::{Client, Error, NoTls};
use redis::{Commands, Connection, FromRedisValue, RedisResult};
use std::num::NonZeroUsize;
use std::process::Command;
use std::time::Duration;
use std::{str, thread};
use std::{env, task};

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

fn main() -> RedisResult<()> {
    println!("Hello, world!");
    let redis_result = get_redis_con();
    let postgres_result = postgres();

    let mut redis_con = redis_result.unwrap();

    let event = Event {
        uid: 1,
        name: "test".to_string(),
        description: "test".to_string(),
        date: "test".to_string(),
        time: "test".to_string(),
        file: "./testcases/task.sh".to_string(),
    };

    // redis_con.lpush::<_, _, ()>("test", event.file).unwrap();
    redis_con.rpush("test", event.file)?;

    loop {
        let task: Option<redis::Value> = redis_con.lpop("test", Default::default())?;
        match task {
            Some(value) => {
                let popped_value: String = FromRedisValue::from_redis_value(&value)?;
                println!("Task: {}", popped_value);
            }
            None => {
                println!("No task ");
                // sleep for 1 second
            }
        }
        thread::sleep(Duration::from_millis(100));
    }


    // let event: Event = Event {
    //     uid: 1,
    //     name: "test".to_string(),
    //     description: "test".to_string(),
    //     date: "test".to_string(),
    //     time: "test".to_string(),
    //     file: "".to_string(),
    // };

    // let action: Action = Action {
    //     uid: 1,
    //     name: "test".to_string(),
    //     description: "test".to_string(),
    //     date: "test".to_string(),
    //     time: "test".to_string(),
    //     file: "test".to_string(),
    // };

    // let mut event_store = EventStore {
    //     events : Vec::new(),
    //     actions: Vec::new()
    // };

    // event_store.events.push(event);
    // event_store.actions.push(action);

    // engine(event_store);
}

fn postgres() -> Result<Client, Error> {
    dotenv().ok();
    let postgres_password = env::var("POSTGRES_PASSWORD").expect("POSTGRES_PASSWORD not set");
    let mut client = Client::connect(
        format!(
            "host=localhost user=postgres password={}",
            postgres_password
        )
        .as_str(),
        NoTls,
    )?;
    print!("Connected to postgres");
    Ok(client)
    // client.batch_execute(
    //     "
    //     CREATE TABLE person (
    //         id      SERIAL PRIMARY KEY,
    //         name    TEXT NOT NULL,
    //         data    BYTEA
    //     )
    // ",
    // )?;

    // let name = "Ferris";
    // let data = None::<&[u8]>;
    // client.execute(
    //     "INSERT INTO person (name, data) VALUES ($1, $2)",
    //     &[&name, &data],
    // )?;

    // for row in client.query("SELECT id, name, data FROM person", &[])? {
    //     let id: i32 = row.get(0);
    //     let name: &str = row.get(1);
    //     let data: Option<&[u8]> = row.get(2);

    //     println!("found person: {} {} {:?}", id, name, data);
    // }
    // Ok(())
}

fn get_redis_con() -> RedisResult<redis::Connection> {
    // connect to redis
    let client = redis::Client::open("redis://172.17.0.2/")?;
    let mut con = client.get_connection()?;
    // throw away the result, just make sure it does not fail
    // let _: () = con.set("my_key", 42)?;
    // read back the key and return it.  Because the return value
    // from the function is a result for integer this will automatically
    // convert into one.
    // con.get("my_key")
    Ok(con)
}

// fn action_executor(action: Action) {
//     println!("Action Executor");
//     let output = Command::new("ls")
//         .arg("a")
//         .arg(action.file)
//         .output()
//         .expect("failed to execute process");

//     println!("status: {}", output.status);
//     println!("stdout: {}", str::from_utf8(&output.stdout).unwrap());
//     println!("stderr: {}", str::from_utf8(&output.stderr).unwrap());
// }

// fn executor(file_path: String) {
//     println!("executing  file: {}", file_path);
//     let output = Command::new("ls")
//         .arg("a")
//         .arg(file_path)
//         .output()
//         .expect("failed to execute process");

//     println!("status: {}", output.status);
//     println!("stdout: {}", str::from_utf8(&output.stdout).unwrap());
//     println!("stderr: {}", str::from_utf8(&output.stderr).unwrap());
// }

// fn engine(event_store: EventStore) {
//     println!("Engine");

//     loop {
//         for event in event_store.events {
//             println!("Event: {}", event.name);

//             if executor(event) != None {
//                 println!("Successfully executed event: {}", event.name);
//             }
//         }
//         // sleep for 1 second
//         std::thread::sleep(std::time::Duration::from_secs(1));
//     }
// }
