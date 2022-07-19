use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use structopt::StructOpt;
use tokio::net::UdpSocket;

use crate::resolver::Resolver;
use crate::server::Listener;

mod resolver;
mod server;

#[derive(Debug, StructOpt, Copy, Clone)]
#[structopt(name = "DNS Server", about = "An example of StructOpt usage.")]
struct ServerOptions {
    #[structopt(short, long)]
    bind_addr: SocketAddr,
    #[structopt(long)]
    no_recursive: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Enable tracing logging to console.
    tracing_subscriber::fmt::try_init().unwrap();

    let opt = ServerOptions::from_args();

    // Create an UDP socket and bind it to the given bind address.
    let socket = UdpSocket::bind(opt.bind_addr).await?;
    let resolver = Resolver::builder().recursive(!opt.no_recursive).build();


    let listener = Listener {
        socket: Arc::new(socket),
        resolver: Arc::new(resolver),
    };

    listener.run().await
}
