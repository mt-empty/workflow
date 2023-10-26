use anyhow::Error as AnyError;
use diesel::prelude::*;
use std::path::Path;
use std::process::Command as ShellCommand;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::{env, str, thread};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, Stream};
use tonic::transport::Server;
use tonic::{Request, Response, Status, Streaming};
use workflow::engine_utils::run_process;
use workflow::models::{EventStatus, LightEvent, LightTask, ProcessStatus};
use workflow::schema;
use workflow::utils::{establish_pg_connection, push_tasks_to_queue};

pub mod grpc {
    tonic::include_proto!("grpc");
}
use grpc::output_streaming_server::{OutputStreaming, OutputStreamingServer};
use grpc::{OutputChunk, Response as GrpcResponse};

#[derive(Debug)]
pub struct OutputStreamer {
    // an stdout pipe steam
    features: Arc<Vec<GrpcResponse>>,
}

#[tonic::async_trait]
impl OutputStreaming for OutputStreamer {
    type StreamOutputStream = ReceiverStream<Result<GrpcResponse, Status>>;

    async fn stream_output(
        &self,
        request: Request<OutputChunk>,
    ) -> Result<Response<Self::StreamOutputStream>, Status> {
        let (mut tx, rx) = mpsc::channel(4);
        let features = self.features.clone();

        // Spawn an async task to send the output data to the client

        tokio::spawn(async move {
            for feature in &features[..] {
                tx.send(Ok(feature.clone())).await.unwrap();
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

pub fn poll_events(running: Arc<AtomicBool>, engine_uid: i32) -> Result<(), AnyError> {
    let mut event_uids: Vec<i32> = Vec::new();
    let pg_conn = &mut establish_pg_connection();

    use workflow::schema::engines::dsl::*;

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
    use workflow::schema::events::dsl::*;
    if output.status.code().unwrap() == 0 {
        diesel::update(events.find(event.uid))
            .set(status.eq(EventStatus::Succeeded.to_string()))
            .execute(conn)?;

        {
            use workflow::schema::tasks::dsl::*;
            let light_tasks: Vec<LightTask> = tasks
                .select(LightTask::as_select())
                .filter(event_uid.eq(event.uid))
                .load(conn)?;
            let _ = push_tasks_to_queue(light_tasks);
        }
    } else {
        diesel::update(events.find(event.uid))
            .set((
                status.eq(EventStatus::Retrying.to_string()),
                triggered_at.eq(diesel::dsl::now),
            ))
            .execute(conn)?;
    };

    diesel::update(schema::events::dsl::events.find(event.uid))
        .set((
            stdout.eq(str::from_utf8(&output.stdout)?),
            stderr.eq(str::from_utf8(&output.stderr)?),
        ))
        .execute(conn)?;

    println!(
        "event id: {} , trigger: {}\nFinished executing with a status: {}",
        event.uid, event.trigger, output.status
    );
    println!("##############################################");
    println!("stdout: {}", str::from_utf8(&output.stdout)?);
    println!("----------------------------------------------");
    println!("stderr: {}", str::from_utf8(&output.stderr)?);
    println!("##############################################");
    Ok(())
}

pub fn run_event_process(engine_uid: i32) -> Result<(), AnyError> {
    run_process("Event", poll_events, engine_uid)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    println!("args: {:?}", args);

    let engine_uid = args[1].parse::<i32>().unwrap();
    println!("engine_uid: {}", engine_uid);

    tokio::spawn(async move {
        if let Err(e) = run_event_process(engine_uid) {
            println!("Failed to start event process, {}", e);
            std::process::exit(1);
        };
    });

    let addr = "[::1]:10001".parse().unwrap();

    let stream = OutputStreamer {
        features: Arc::new(vec![GrpcResponse {
            message: "Hello from event".into(),
        }]),
    };

    let svc = OutputStreamingServer::new(stream);

    Server::builder().add_service(svc).serve(addr).await?;

    Ok(())
}
