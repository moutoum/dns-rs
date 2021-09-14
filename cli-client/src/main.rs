extern crate dns;

use std::env::args;
use std::net::{SocketAddr, UdpSocket};

use dns::byte_packet_buffer::BytePacketBuffer;
use dns::header::{Header, OpCode, ResultCode};
use dns::packet::{Packet, Question, QueryType};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let qname = args().nth(1).ok_or("missing qname")?;
    let server = "8.8.8.8:53";
    let socket = UdpSocket::bind("0.0.0.0:43053")?;

    let mut header = Header::new();
    header.id = 9876;
    header.is_response = false;
    header.opcode = OpCode::Query;
    header.recursion_desired = true;
    header.total_questions = 1;

    let packet = Packet {
        header,
        questions: vec![Question{
            name: qname,
            qtype: QueryType::A,
            _class: 1
        }],
        answers: vec![],
        authorities: vec![],
        additionals: vec![],
    };

    let mut buf = BytePacketBuffer::new();
    packet.write_to_buffer(&mut buf);
    socket.send_to(&buf.bytes(), server)?;

    let mut data= [0u8; 512];
    socket.recv(&mut data)?;
    let mut buffer = BytePacketBuffer::from_raw_data(&data);
    let packet = Packet::from_buffer(&mut buffer)?;

    println!("{:#?}", packet);

    Ok(())
}
