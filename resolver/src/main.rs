use std::net::{SocketAddr, UdpSocket};

use anyhow::Result;
use structopt::StructOpt;

use protocol::byte_packet_buffer::BytePacketBuffer;
use protocol::packet::Packet;
use protocol::ser::Serialize;

use crate::resolver::Resolver;

mod resolver;

#[derive(Debug, StructOpt)]
#[structopt(name = "DNS Server", about = "An example of StructOpt usage.")]
struct ServerOptions {
    #[structopt(short, long)]
    bind_addr: SocketAddr,
    #[structopt(long)]
    no_recursive: bool,
}


fn main() -> Result<()> {
    // Starting udp server to receive DNS requests.
    let opt = ServerOptions::from_args();
    let socket = UdpSocket::bind(&opt.bind_addr)?;
    println!("Starting server on {}", opt.bind_addr);

    // Reading socket.
    let mut data = [0u8; 512];
    let (_, src) = socket.recv_from(&mut data)?;

    // Parsing request data into DNS Packet.
    let mut buffer = BytePacketBuffer::from_raw_data(&data);
    let mut request = Packet::from_buffer(&mut buffer);

    // Creating response DNS Packet based on the request.
    let mut response = Packet::new();

    // Taking the first question and resolve it. Maybe considering
    // looping over all the questions in the future.
    if let Some(question) = request.questions.pop() {

        // For now i'm only using the first root server but
        // a better idea would be to randomly select the server
        // from the root server list.
        // let root_server = &ROOT_SERVERS[0];
        // let server_ip = Ipv4Addr::from(root_server.1);
        // response = recursive_lookup(&question.name, question.qtype, server_ip, opt.no_recursive)?;

        let resolver = Resolver::builder().recursive(true).build();
        response = resolver.resolve(question.name, question.qtype)?;
    }

    response.header.id = request.header.id;
    response.header.recursion_desired = request.header.recursion_desired;
    response.header.recursion_available = true;
    response.header.is_response = true;

    let mut buffer = BytePacketBuffer::new();
    response.serialize(&mut buffer)?;
    socket.send_to(&buffer.bytes(), src)?;

    Ok(())
}