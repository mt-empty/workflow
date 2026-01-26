use anyhow::Error as AnyError;
use bincode::deserialize;
use diesel::prelude::*;
use rand::seq::IteratorRandom;
use rayon::ThreadPoolBuilder;
use redis::{Commands as RedisCommand, FromRedisValue};
use tonic::server::ServerStreamingService;
// use std::io::BufReader;
use std::path::Path;
use std::pin::Pin;
use std::process::{ChildStdout, Command as ShellCommand, Stdio};
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
use workflow::models::{LightTask, ProcessStatus, TaskStatus};
use workflow::utils::{self, create_redis_connection, establish_pg_connection};

pub mod grpc {
    tonic::include_proto!("grpc");
}
// use grpc::output_streaming_client::OutputStreamingClient;
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

const THREAD_COUNT: usize = 4;

pub fn queue_processor(running: Arc<AtomicBool>, engine_uid: i32) -> Result<(), AnyError> {
    let thread_pool = ThreadPoolBuilder::new().num_threads(THREAD_COUNT).build()?;
    let pg_conn = &mut establish_pg_connection();
    let mut redis_con = create_redis_connection()?;

    use workflow::schema::engines::dsl::*;

    diesel::update(engines)
        .filter(uid.eq(engine_uid))
        .set(task_process_status.eq(ProcessStatus::Running.to_string()))
        .execute(pg_conn)?;

    while running.load(Ordering::SeqCst) {
        let task: Option<redis::Value> = redis_con.lpop(utils::QUEUE_NAME, Default::default())?;
        match task {
            Some(value) => {
                let popped_value: String = FromRedisValue::from_redis_value(&value)?;
                // If the program exists, then thread_pool will be dropped and all threads will be stopped
                // which means that threads will not be able to complete their current task
                thread_pool.spawn(move || {
                    let task: LightTask = deserialize(popped_value.as_bytes()).unwrap();
                    println!("Task: {}", task);
                    let future = execute_task(task);
                    // if let Err(e) = execute_task(task) {
                    //     println!("Failed to execute task {}", e);
                    // };
                });
            }
            None => {
                println!("No task to process");
            }
        }

        // rust couldn't infer the type of received_stop_signal_result
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

        thread::sleep(Duration::from_millis(2000));
    }

    if !running.load(Ordering::SeqCst) {
        println!("\nCtrl+C signal detected. Exiting...");
    }

    diesel::update(engines)
        .filter(uid.eq(engine_uid))
        .set(task_process_status.eq(ProcessStatus::Stopped.to_string()))
        .execute(pg_conn)?;
    Ok(())
}

async fn execute_task(task: LightTask) -> Result<(), AnyError> {
    println!("Task Executor");

    use workflow::schema::tasks::dsl::*;
    let conn = &mut establish_pg_connection();
    // let mut client = OutputStreamingClient::connect("http://[::1]:50051").await?;
    diesel::update(tasks.find(task.uid))
        .set((
            status.eq(TaskStatus::Running.to_string()),
            updated_at.eq(diesel::dsl::now),
        ))
        .execute(conn)?;

    let path_basename = match Path::new(&task.path).file_name() {
        Some(basename) => basename,
        None => return Err(AnyError::msg("Failed to get path basename")),
    };
    let path_dirname = Path::new(&task.path).parent().unwrap();

    let mut cmd = ShellCommand::new("bash")
        .arg(path_basename)
        .current_dir(path_dirname)
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to execute process");

    let stdout_content = cmd
        .stdout
        .take()
        .expect("Could not capture standard output");
    // let reader = tokio::io::BufReader::new(stdout_content.into());

    // let mut client_stream = client
    //     .stream_output(Request::new(OutputChunk {
    //         content: "Tonic".into(),
    //     }))
    //     .await?
    //     .into_inner();

    // tokio::spawn(async move {
    //     let mut buf = String::new();
    //     let mut reader = reader;
    //     loop {
    //         buf.clear();
    //         if reader.read_line(&mut buf).await.unwrap_or(0) == 0 {
    //             break;
    //         }

    //         let request = Request::new(grpc::OutputChunk {
    //             content: buf.clone(),
    //         });

    //         if let Err(_) = client_stream.send(request).await {
    //             break;
    //         }
    //     }
    // });

    let output = cmd.wait_with_output().expect("Failed to wait for command");

    println!(
        "task id: {} , path: {}\nFinished executing with a status: {}",
        task.uid, task.path, output.status
    );

    // let stdout_content = str::from_utf8(stdout_content)?;

    // for chunk in stdout_content.chars().collect::<Vec<char>>().chunks(10) {
    //     let request = tonic::Request::new(mygrpc::OutputChunk {
    //         content: chunk.iter().collect(),
    //     });

    //     client.stream_output(request).await?;
    // }

    if output.status.code().unwrap() == 0 {
        diesel::update(tasks.find(task.uid))
            .set((
                status.eq(TaskStatus::Completed.to_string()),
                updated_at.eq(diesel::dsl::now),
                completed_at.eq(diesel::dsl::now),
            ))
            .execute(conn)?;

        // TODO run on failure
    } else {
        diesel::update(tasks.find(task.uid))
            .set((
                status.eq(TaskStatus::Failed.to_string()),
                updated_at.eq(diesel::dsl::now),
            ))
            .execute(conn)?;
    }

    // Write stdout and stderr to the database
    diesel::update(tasks.find(task.uid))
        .set((
            stdout.eq(str::from_utf8(&output.stdout)?),
            stderr.eq(str::from_utf8(&output.stderr)?),
        ))
        .execute(conn)?;

    println!(
        "task id: {} , path: {}\nFinished executing with a status: {}",
        task.uid, task.path, output.status
    );
    println!("##############################################");
    println!("stdout: {}", str::from_utf8(&output.stdout)?);
    println!("----------------------------------------------");
    println!("stderr: {}", str::from_utf8(&output.stderr)?);
    println!("##############################################");
    Ok(())
}

pub fn run_task_process(engine_uid: i32) -> Result<(), AnyError> {
    run_process("Task", queue_processor, engine_uid)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    println!("args: {:?}", args);

    let engine_uid = args[1].parse::<i32>().unwrap();
    println!("engine_uid: {}", engine_uid);

    tokio::spawn(async move {
        if let Err(e) = run_task_process(engine_uid) {
            println!("Failed to start event process, {}", e);
            std::process::exit(1);
        };
    });

    let addr = "[::1]:10000".parse().unwrap();

    let stream = OutputStreamer {
        features: Arc::new(vec![GrpcResponse {
            message: "Hello from task".into(),
        }]),
    };

    let svc = OutputStreamingServer::new(stream);

    Server::builder().add_service(svc).serve(addr).await?;

    Ok(())
}
