use anyhow::Error as AnyError;
use std::env;
use workflow::components::event::poll_events;
use workflow::components::task::queue_processor;
use workflow::engine_utils::run_process;
// mod models;
// mod utils;

use diesel::prelude::*;

// I want to use the poll_events function from src/event.rs

use tonic::{transport::Server, Request, Response, Status};

use grpc::greeter_server::{Greeter, GreeterServer};
use grpc::{LogMessageRequest, LogMessageResponse};

pub mod grpc {
    tonic::include_proto!("grpc");
}

// pub fn run_task_process(engine_uid: i32) -> Result<(), AnyError> {
//     run_process("Task", queue_processor, engine_uid)
// }

// pub fn run_event_process(engine_uid: i32) -> Result<(), AnyError> {
//     run_process("Event", poll_events, engine_uid)
// }

#[derive(Debug, Default)]
pub struct MyGreeter {}

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<LogMessageRequest>,
    ) -> Result<Response<LogMessageResponse>, Status> {
        println!("Got a request: {:?}", request);

        let reply = grpc::LogMessageResponse {
            message: format!("Hello {}!", request.into_inner().content).into(),
        };

        Ok(Response::new(reply))
    }
}

//main function that takes an argument
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    println!("args: {:?}", args);

    let engine_uid = args[1].parse::<i32>().unwrap();
    println!("engine_uid: {}", engine_uid);

    // tokio::spawn(async move {
    //     if let Err(e) = run_event_process(engine_uid) {
    //         println!("Failed to start event process, {}", e);
    //         std::process::exit(1);
    //     };
    // });

    // tokio::spawn(async move {
    //     if let Err(e) = run_task_process(engine_uid) {
    //         println!("Failed to start event process, {}", e);
    //         std::process::exit(1);
    //     };
    // });

    let addr = "[::1]:50051".parse()?;
    let greeter = MyGreeter::default();

    Server::builder()
        .add_service(GreeterServer::new(greeter))
        .serve(addr)
        .await?;

    Ok(())
}
