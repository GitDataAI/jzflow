#![feature(trait_alias)]
#![feature(async_closure)]
#![feature(future_join)]

pub mod core;
pub mod dag;
pub mod dbrepo;
pub mod driver;
pub mod network;
pub mod utils;
pub mod rpc;

/*
use dag::Dag;

use network::helloworld::greeter_server::{Greeter, GreeterServer};
use network::helloworld::{HelloReply, HelloRequest};
use tonic::{transport::Server, Request, Response, Status};

#[derive(Default)]
pub struct MyGreeter {}

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        println!("Got a request from {:?}", request.remote_addr());

        let reply = network::helloworld::HelloReply {
            message: format!("Hello {}!", request.into_inner().name),
        };
        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse().unwrap();
    let greeter = MyGreeter::default();

    println!("GreeterServer listening on {}", addr);

    Server::builder()
        .add_service(GreeterServer::new(greeter))
        .serve(addr)
        .await?;

    Ok(())
}
 */
