use anyhow::Result;
use structopt::StructOpt;

use protocol::byte_packet_buffer::BytePacketBuffer;
use protocol::packet::Packet;
use protocol::ser::Serialize;

use crate::resolver::Resolver;
use tokio::net::UdpSocket;
use std::net::SocketAddr;

mod resolver;

#[derive(Debug, StructOpt)]
#[structopt(name = "DNS Server", about = "An example of StructOpt usage.")]
struct ServerOptions {
    #[structopt(short, long)]
    bind_addr: SocketAddr,
    #[structopt(long)]
    no_recursive: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Starting udp server to receive DNS requests.
    let opt = ServerOptions::from_args();
    let socket = UdpSocket::bind(&opt.bind_addr).await?;
    println!("Starting server on {}", opt.bind_addr);

    loop {
        // Creating buffer and waiting for incoming packets.
        // When received, connect the socket to the incoming
        // ip address.
        let mut data = [0u8; 512];
        let (len, src) = socket.recv_from(&mut data).await?;
        socket.connect(src).await?;

        // Parsing request data into DNS Packet.
        let mut buffer = BytePacketBuffer::from_raw_data(&data[..len]);
        let mut request = Packet::from_buffer(&mut buffer);

        // Creating response DNS Packet based on the request.
        let mut response = Packet::new();

        // Taking the first question and resolve it. Maybe considering
        // looping over all the questions in the future.
        if let Some(question) = request.questions.pop() {

            response = Resolver::builder()
                .recursive(!opt.no_recursive)
                .build()
                .resolve(question.name, question.qtype, request.header.recursion_desired)?;
        }

        response.header.id = request.header.id;
        response.header.recursion_desired = request.header.recursion_desired;
        response.header.recursion_available = !opt.no_recursive;
        response.header.is_response = true;

        let mut buffer = BytePacketBuffer::new();
        response.serialize(&mut buffer)?;
        socket.send(&buffer.bytes()).await?;
    }
}