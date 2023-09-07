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
    // TODO: an stdout pipe steam
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

fn run_event_process(engine_uid: i32) -> Result<(), AnyError> {
    println!("run_event_process");
    let path_basename = Path::new(&env::current_exe().unwrap())
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let mut cmd = ShellCommand::new("bash")
        .arg(path_basename)
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to execute process");

    // TODO: an stdout pipe steam

    Ok(())
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
