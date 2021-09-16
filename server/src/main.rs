use std::net::{Ipv4Addr, SocketAddr, UdpSocket};

use anyhow::Result;
use structopt::StructOpt;

use dns::byte_packet_buffer::BytePacketBuffer;
use dns::header::{Header, OpCode, ResultCode};
use dns::packet::{Packet, QueryType, Question, Record};

#[derive(Debug, StructOpt)]
#[structopt(name = "DNS Server", about = "An example of StructOpt usage.")]
struct ServerOptions {
    #[structopt(short, long)]
    bind_addr: SocketAddr,
    #[structopt(long)]
    no_recursive: bool,
}

// https://www.internic.net/domain/named.root
const ROOT_SERVERS: &[(&str, [u8; 4])] = &[
    ("a.root-servers.net", [198, 41, 0, 4]),
    ("b.root-servers.net", [199, 9, 14, 201]),
    ("c.root-servers.net", [192, 33, 4, 12]),
    ("d.root-servers.net", [199, 7, 91, 13]),
    ("e.root-servers.net", [192, 203, 230, 10]),
    ("f.root-servers.net", [192, 5, 5, 241]),
    ("g.root-servers.net", [192, 112, 36, 4]),
    ("h.root-servers.net", [198, 97, 190, 53]),
    ("i.root-servers.net", [192, 36, 148, 17]),
    ("j.root-servers.net", [192, 58, 128, 30]),
    ("k.root-servers.net", [193, 0, 14, 129]),
    ("l.root-servers.net", [199, 7, 83, 42]),
    ("m.root-servers.net", [202, 12, 27, 33]),
];

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
        let root_server = &ROOT_SERVERS[0];
        let server_ip = Ipv4Addr::from(root_server.1);
        response = recursive_lookup(&question.name, question.qtype, server_ip, opt.no_recursive)?;
    }

    response.header.id = request.header.id;
    response.header.recursion_desired = request.header.recursion_desired;
    response.header.recursion_available = true;
    response.header.is_response = true;

    let mut buffer = BytePacketBuffer::new();
    response.write_to_buffer(&mut buffer);
    socket.send_to(&buffer.bytes(), src)?;

    Ok(())
}

fn recursive_lookup(qname: &str, qtype: QueryType, server_ip: Ipv4Addr, no_recursive: bool) -> Result<Packet> {
    let mut server_ip = server_ip;

    loop {
        println!("Looking up of {:?} for {} with {}", qtype, qname, server_ip);

        let response = lookup(qname, qtype, server_ip)?;

        // If we received some answers and the result code is ok then we found
        // a match for the query.
        if !response.answers.is_empty() && response.header.result_code == ResultCode::NoError {
            return Ok(response);
        }

        // NXDomain means that the authoritative server doesn't know
        // the queried domain. In this case we are just returning an
        // error to the user.
        if response.header.result_code == ResultCode::NXDomain {
            return Err(anyhow::anyhow!("The requested domain {} does not exist", qname));
        }

        // When the --no-recursive option is enabled, we are not
        // looping over the authoritative servers to find an answer.
        // Instead, we are just displaying the latest response.
        if no_recursive {
            return Ok(response);
        }

        // + Find NS records corresponding to queried domain.
        let mut matching_ns = find_matching_ns(qname, &response);

        // + Check if one of the NS record has an additional A record.
        if let Some(ip) = find_matching_ns_a(qname, &response) {
            server_ip = ip;
            continue;
        }

        // + Perform new request to the same server for a random NS.
        let authoritative_name_server = match matching_ns {
            Some((_, ns)) => ns,
            None => return Ok(response),
        };

        let ns_response = recursive_lookup(authoritative_name_server, QueryType::A, server_ip, no_recursive)?;

        // println!("ns_response: {:#?}", ns_response);

        // + Once the authoritative server ip is found, continue the loop
        //   with the new server for the queried domain.
        let fist_answer = ns_response.answers
            .iter()
            .filter_map(|record| match record {
                Record::A { ip, .. } => Some(ip),
                _ => None
            })
            .next();

        server_ip = match fist_answer {
            Some(ip) => ip.clone(),
            None => return Ok(response),
        }
    }
}

fn find_matching_ns<'a>(qname: &'a str, packet: &'a Packet) -> Option<(&'a str, &'a str)> {
    packet.authorities
        .iter()
        .filter_map(|record| match record {
            Record::AuthoritativeNameServer { domain, ns_name, .. } => Some((domain.as_str(), ns_name.as_str())),
            _ => None
        })
        .filter(move |(domain, _)| qname.ends_with(domain))
        .next()
}

fn find_matching_ns_a<'a>(qname: &'a str, packet: &'a Packet) -> Option<Ipv4Addr> {
    packet.authorities
        .iter()
        .filter_map(|record| match record {
            Record::AuthoritativeNameServer { domain, ns_name, .. } => Some((domain.as_str(), ns_name.as_str())),
            _ => None,
        })
        .filter(move |(domain, _)| qname.ends_with(domain))
        .flat_map(|(_, host)|
            packet.additionals
                .iter()
                .filter_map(move |record| match record {
                    Record::A { ip, domain, .. } if domain == host => Some(ip),
                    _ => None,
                })
        )
        .map(|ip| *ip)
        .next()
}

fn lookup(qname: &str, qtype: QueryType, server_ip: Ipv4Addr) -> Result<Packet> {
    let server_endpoint = SocketAddr::from((server_ip, 53));
    let socket = UdpSocket::bind("0.0.0.0:43053")?;

    let mut header = Header::new();
    header.id = 9876;
    header.is_response = false;
    header.opcode = OpCode::Query;
    header.recursion_desired = true;
    header.total_questions = 1;

    let packet = Packet {
        header,
        questions: vec![Question {
            name: qname.to_string(),
            qtype,
            _class: 1,
        }],
        answers: vec![],
        authorities: vec![],
        additionals: vec![],
    };

    let mut buf = BytePacketBuffer::new();
    packet.write_to_buffer(&mut buf);
    socket.send_to(&buf.bytes(), server_endpoint)?;

    let mut data = [0u8; 512];
    let s = socket.recv(&mut data)?;
    let mut buffer = BytePacketBuffer::from_raw_data(&data);
    Ok(Packet::from_buffer(&mut buffer))
}