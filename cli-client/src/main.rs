extern crate dns;

use std::env::args;

use dns::byte_packet_buffer::BytePacketBuffer;
use dns::header::Header;
use dns::packet::Packet;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = args().nth(1).ok_or("missing file path argument")?;
    let data = std::fs::read(path)?;
    let mut buffer = BytePacketBuffer::from_raw_data(&data);
    let packet = Packet::from_buffer(&mut buffer)?;

    println!("{:#?}", packet);

    Ok(())
}
