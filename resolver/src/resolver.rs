use std::cell::Cell;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};

use anyhow::Result;
use rand::prelude::ThreadRng;
use rand::Rng;

use protocol::byte_packet_buffer::BytePacketBuffer;
use protocol::header::{Header, OpCode, ResultCode};
use protocol::packet::{Packet, QueryType, Question, Record};

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

pub struct Resolver {
    recursive: bool,
    root_servers: Vec<(String, IpAddr)>,
    rng: Cell<ThreadRng>,
}

impl Resolver {
    pub fn new() -> Resolver {
        Resolver {
            recursive: false,
            root_servers: vec![],
            rng: Cell::new(rand::thread_rng()),
        }
    }

    pub fn builder() -> ResolverBuilder {
        ResolverBuilder::new()
    }

    pub fn resolve<S>(&self, qname: S, qtype: QueryType) -> Result<Packet>
        where S: AsRef<str>
    {
        let (_, addr) = self.get_root_server();
        self.recursive_lookup(qname, qtype, *addr)
    }

    fn get_root_server(&self) -> &(String, IpAddr) {
        &self.root_servers[0]
    }

    fn recursive_lookup<S>(&self, qname: S, qtype: QueryType, server_ip: IpAddr) -> Result<Packet>
        where S: AsRef<str>
    {
        let mut server_ip = server_ip;

        loop {
            println!("Looking up of {:?} for {} with {}", qtype, qname.as_ref(), server_ip);

            let response = self.lookup(&qname, qtype, server_ip)?;

            // If we received some answers and the result code is ok then we found
            // a match for the query.
            if !response.answers.is_empty() && response.header.result_code == ResultCode::NoError {
                return Ok(response);
            }

            // NXDomain means that the authoritative server doesn't know
            // the queried domain. In this case we are just returning an
            // error to the user.
            if response.header.result_code == ResultCode::NXDomain {
                return Err(anyhow::anyhow!("The requested domain {} does not exist", qname.as_ref()));
            }

            // When the --no-recursive option is enabled, we are not
            // looping over the authoritative servers to find an answer.
            // Instead, we are just displaying the latest response.
            if !self.recursive {
                return Ok(response);
            }

            // + Find NS records corresponding to queried domain.
            let matching_ns = self.find_matching_ns(&qname, &response);

            // + Check if one of the NS record has an additional A record.
            if let Some(ip) = self.find_matching_ns_a(&qname, &response) {
                server_ip = IpAddr::V4(ip);
                continue;
            }

            // + Perform new request to the same server for a random NS.
            let authoritative_name_server = match matching_ns {
                Some((_, ns)) => ns,
                None => return Ok(response),
            };

            let ns_response = self.recursive_lookup(authoritative_name_server, QueryType::A, server_ip)?;

            // + Once the authoritative server ip is found, continue the loop
            //   with the new server for the queried domain.
            let fist_answer = ns_response.answers
                .iter()
                .filter_map(|record| match record {
                    Record::A { ip, .. } => Some(IpAddr::V4(*ip)),
                    _ => None
                })
                .next();

            server_ip = match fist_answer {
                Some(ip) => ip,
                None => return Ok(response),
            }
        }
    }

    fn find_matching_ns<'a, S>(&self, qname: S, packet: &'a Packet) -> Option<(&'a str, &'a str)>
        where S: AsRef<str>
    {
        packet.authorities
            .iter()
            .filter_map(|record| match record {
                Record::AuthoritativeNameServer { domain, ns_name, .. } => Some((domain.as_str(), ns_name.as_str())),
                _ => None
            })
            .find(|(domain, _)| qname.as_ref().ends_with(domain))
    }

    fn find_matching_ns_a<S>(&self, qname: S, packet: &Packet) -> Option<Ipv4Addr>
        where S: AsRef<str>
    {
        packet.authorities
            .iter()
            .filter_map(|record| match record {
                Record::AuthoritativeNameServer { domain, ns_name, .. } => Some((domain.as_str(), ns_name.as_str())),
                _ => None,
            })
            .filter(move |(domain, _)| qname.as_ref().ends_with(domain))
            .flat_map(|(_, host)|
                packet.additionals
                    .iter()
                    .filter_map(move |record| match record {
                        Record::A { ip, domain, .. } if domain == host => Some(ip),
                        _ => None,
                    })
            )
            .copied()
            .next()
    }

    fn lookup<S>(&self, qname: S, qtype: QueryType, server_ip: IpAddr) -> Result<Packet>
        where S: AsRef<str>
    {
        let server_endpoint = SocketAddr::from((server_ip, 53));
        let socket = UdpSocket::bind("0.0.0.0:43053")?;

        let query = Query::new(self.get_random_id(), qname.as_ref(), qtype, true);
        let mut buf = BytePacketBuffer::new();
        query.write_to_buffer(&mut buf);
        socket.send_to(&buf.bytes(), server_endpoint)?;

        let mut data = [0u8; 512];
        socket.recv(&mut data)?;
        let mut buffer = BytePacketBuffer::from_raw_data(&data);
        Ok(Packet::from_buffer(&mut buffer))
    }

    fn get_random_id(&self) -> u16 {
        let mut rng = self.rng.take();
        let id = rng.gen();
        self.rng.set(rng);
        id
    }
}

pub struct ResolverBuilder {
    recursive: bool,
    root_servers: Vec<(String, IpAddr)>,
}

impl ResolverBuilder {
    pub fn new() -> Self {
        ResolverBuilder {
            recursive: true,
            root_servers: ROOT_SERVERS
                .iter()
                .map(|(domain, addr)| (domain.to_string(), IpAddr::V4(Ipv4Addr::from(*addr))))
                .collect(),
        }
    }

    pub fn recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }

    pub fn build(self) -> Resolver {
        let mut resolver = Resolver::new();
        resolver.recursive = self.recursive;
        resolver.root_servers = self.root_servers;
        resolver
    }
}

struct Query {
    packet: Packet,
}

impl Query {
    fn new<S>(id: u16, qname: S, qtype: QueryType, recursion_desired: bool) -> Query
        where S: ToString
    {
        Query {
            packet: Packet {
                header: Header {
                    id,
                    is_response: false,
                    opcode: OpCode::Query,
                    authoritative_answer: false,
                    truncated: false,
                    recursion_desired,
                    recursion_available: false,
                    z: false,
                    authenticated_data: true,
                    checking_disabled: false,
                    result_code: ResultCode::NoError,
                    total_questions: 1,
                    total_answer_records: 0,
                    total_authority_records: 0,
                    total_additional_records: 0,
                },
                questions: vec![Question {
                    name: qname.to_string(),
                    qtype,
                    _class: 1,
                }],
                answers: vec![],
                authorities: vec![],
                additionals: vec![],
            }
        }
    }

    fn write_to_buffer(self, buf: &mut BytePacketBuffer) {
        self.packet.write_to_buffer(buf)
    }
}