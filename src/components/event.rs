use anyhow::Error as AnyError;
use ctrlc::set_handler;
use diesel::prelude::*;
use std::path::Path;
use std::process::{Command as ShellCommand, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::{env, str, thread};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, DuplexStream};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::{wrappers::ReceiverStream, Stream};
use tonic::transport::Server;
use tonic::{Request, Response, Status, Streaming};
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
    read_pipe: tokio::io::ReadHalf<tokio::io::DuplexStream>,
}

#[tonic::async_trait]
impl OutputStreaming for OutputStreamer {
    type StreamOutputStream = ReceiverStream<Result<GrpcResponse, Status>>;
    async fn stream_output(
        &self, // Make self mutable here
        request: Request<OutputChunk>,
    ) -> Result<Response<Self::StreamOutputStream>, Status> {
        let (mut tx, rx) = mpsc::channel(4);
        let pipe = &self.read_pipe;
        tokio::spawn(async move {
            let mut buf = String::new();
            loop {
                let n = pipe.read_to_string(&mut buf).await;
                match n {
                    Ok(n) => {
                        if n == 0 {
                            break;
                        }
                    }
                    Err(e) => {
                        println!("Error reading from pipe: {}", e);
                        break;
                    }
                };
                tx.send(Ok(GrpcResponse {
                    message: buf.clone(),
                }))
                .await
                .unwrap();
                buf.clear();
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

pub fn poll_events(
    running: Arc<AtomicBool>,
    engine_uid: i32,
    stdout_write: &mut DuplexStream,
) -> Result<(), AnyError> {
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
            stdout_write.write_all(event.trigger.as_bytes());
            // async execute_event
            let _ = execute_event(event);
        }

        if event_uids.is_empty() {
            println!("No events to process");
            stdout_write.write_all(b"No events to process");
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
                    stdout_write.write_all(b"Received stop signal");
                    break;
                }
            }
            Ok(None) => {
                println!("No stop signal");
                stdout_write.write_all(b"No stop signal");
            }
            Err(e) => {
                eprintln!("Failed to query engine status {}", e);
                eprintln!("exiting...");
                stdout_write.write_all(format!("Failed to query engine status {}", e).as_bytes());
                stdout_write.write_all(b"exiting...");
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

pub fn run_process<F>(
    process_name: &str,
    process_fn: F,
    engine_uid: i32,
    stdout_pipe: &mut DuplexStream,
) -> Result<(), AnyError>
where
    F: FnOnce(Arc<AtomicBool>, i32, &mut DuplexStream) -> Result<(), AnyError>,
{
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    if let Err(e) = process_fn(running, engine_uid, stdout_pipe) {
        eprintln!("Failed to start {} process: {}", process_name, e);
        eprintln!("exiting...");
        std::process::exit(1);
    }
    println!("{} process stopped correctly", process_name);

    Ok(())
}

pub fn run_event_process(engine_uid: i32, stdout_pipe: &mut DuplexStream) -> Result<(), AnyError> {
    run_process("Event", poll_events, engine_uid, stdout_pipe)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    println!("args: {:?}", args);

    let engine_uid = args[1].parse::<i32>().unwrap();
    println!("engine_uid: {}", engine_uid);

    // let stdout_pipe: Arc<RwLock<Stdio>> = Arc::new(RwLock::new(Stdio::piped()));
    // let stdout_pipe = Stdio::piped();
    let (stdout_read, mut stdout_write) = tokio::io::duplex(1024);

    // let (read, write) = tokio::io::duplex(1024);
    let (read_rx, _) = tokio::io::split(stdout_read);
    // let (mut write_rx, mut read_tx) = tokio::io::split(stdout_write);

    tokio::spawn(async move {
        if let Err(e) = run_event_process(engine_uid, &mut stdout_write) {
            println!("Failed to start event process, {}", e);
            std::process::exit(1);
        };
    });

    let addr = "[::1]:10001".parse().unwrap();

    let stream = OutputStreamer { read_pipe: read_rx };

    let svc = OutputStreamingServer::new(stream);

    Server::builder().add_service(svc).serve(addr).await?;

    Ok(())
}
