use crate::models::{EventStatus, LightEvent, LightTask, ProcessStatus};
use crate::schema;
use crate::utils::{establish_pg_connection, push_tasks_to_queue};
use anyhow::Error as AnyError;
use diesel::prelude::*;
use std::path::Path;
use std::process::Command as ShellCommand;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::{str, thread};

pub fn poll_events(running: Arc<AtomicBool>, engine_uid: i32) -> Result<(), AnyError> {
    let mut event_uids: Vec<i32> = Vec::new();
    let pg_conn = &mut establish_pg_connection();

    use crate::schema::engines::dsl::*;

    diesel::update(engines)
        .filter(uid.eq(engine_uid))
        .set(event_process_status.eq(ProcessStatus::Running.to_string()))
        .execute(pg_conn)?;

    while running.load(Ordering::SeqCst) {
        let events: Vec<LightEvent> = schema::events::dsl::events
            .select(LightEvent::as_select())
            .filter(schema::events::status.ne(EventStatus::Succeeded.to_string()))
            .load(pg_conn)?;

        for event in events {
            println!("Event: {}", event);
            // async execute_event
            let _ = execute_event(event);
        }

        if event_uids.is_empty() {
            println!("No events to process");
            thread::sleep(Duration::from_millis(2000));
        }

        let received_stop_signal_result: Result<Option<bool>, _> = engines
            .find(engine_uid)
            .select(stop_signal)
            .first(pg_conn)
            .optional();
        match received_stop_signal_result {
            Ok(Some(signal_on)) => {
                if signal_on {
                    println!("Received stop signal");
                    break;
                }
            }
            Ok(None) => {
                println!("No stop signal");
            }
            Err(e) => {
                eprintln!("Failed to query engine status {}", e);
                eprintln!("exiting...");
                std::process::exit(1);
            }
        }

        event_uids.clear();
    }
    if !running.load(Ordering::SeqCst) {
        println!("\nCtrl+C signal detected. Exiting...");
    }

    diesel::update(engines)
        .filter(uid.eq(engine_uid))
        .set(event_process_status.eq(ProcessStatus::Stopped.to_string()))
        .execute(pg_conn)?;
    Ok(())
}

fn execute_event(event: LightEvent) -> Result<(), AnyError> {
    println!("Event Executor");

    let conn = &mut establish_pg_connection();

    let path_basename = match Path::new(&event.trigger).file_name() {
        Some(basename) => basename,
        None => return Err(AnyError::msg("Failed to get path basename")),
    };
    let path_dirname = Path::new(&event.trigger).parent().unwrap();

    let output = ShellCommand::new("bash")
        .arg(path_basename)
        .current_dir(path_dirname)
        .output()
        .expect("failed to execute process");

    // if shell command return 0, then the event was triggered successfully
    if output.status.code().unwrap() == 0 {
        {
            use crate::schema::events::dsl::*;
            diesel::update(events.find(event.uid))
                .set(status.eq(EventStatus::Succeeded.to_string()))
                .execute(conn)?;
        }
        use crate::schema::tasks::dsl::*;

        let light_tasks: Vec<LightTask> = tasks
            .select(LightTask::as_select())
            .filter(event_uid.eq(event.uid))
            .load(conn)?;
        let _ = push_tasks_to_queue(light_tasks);
    } else {
        use crate::schema::events::dsl::*;
        diesel::update(events.find(event.uid))
            .set((
                status.eq(EventStatus::Retrying.to_string()),
                triggered_at.eq(diesel::dsl::now),
            ))
            .execute(conn)?;
    };

    println!("status: {}", output.status);
    // TODO: write to disk
    println!("stdout: {}", str::from_utf8(&output.stdout)?);
    println!("stderr: {}", str::from_utf8(&output.stderr)?);
    Ok(())
}
