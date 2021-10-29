use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};

use anyhow::Result;

use protocol::byte_packet_buffer::BytePacketBuffer;
use protocol::header::{Header, OpCode, ResultCode};
use protocol::packet::{Packet, QueryType, Question, Record};
use protocol::ser::Serialize;

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
    pub(crate) recursive: bool,
    root_servers: Vec<(String, IpAddr)>,
}

impl Resolver {
    pub fn new() -> Resolver {
        Resolver {
            recursive: false,
            root_servers: vec![],
        }
    }

    pub fn builder() -> ResolverBuilder {
        ResolverBuilder::new()
    }

    pub fn resolve<S>(&self, qname: S, qtype: QueryType, recursion_desired: bool) -> Result<Packet>
        where S: AsRef<str>
    {
        let (name, addr) = self.get_root_server();
        println!("Start {} resolution with {} ({})", qname.as_ref(), name, addr);
        self.recursive_lookup(qname, qtype, *addr, recursion_desired)
    }

    fn get_root_server(&self) -> &(String, IpAddr) {
        let index = rand::random::<usize>() % self.root_servers.len();
        &self.root_servers[index]
    }

    fn recursive_lookup<S>(&self, qname: S, qtype: QueryType, server_ip: IpAddr, recursion_desired: bool) -> Result<Packet>
        where S: AsRef<str>
    {
        let mut server_ip = server_ip;

        loop {
            println!("Looking up of {:?} for {} with {}", qtype, qname.as_ref(), server_ip);

            let response = self.lookup(&qname, qtype, server_ip)?;

            // If we received some answers and the result code is ok then we found
            // a match for the query.
            // TODO: Currently, if the answer contains only CNAMEs, the response will not
            //       be complete. To make it fully usable, it needs to recursively resolve
            //       the CNAME alias to match the query type.
            if !response.answers.is_empty() && response.header.result_code == ResultCode::NoError {
                return Ok(response);
            }

            // NXDomain means that the authoritative server doesn't know
            // the queried domain. In this case we are just returning an
            // error to the user.
            if response.header.result_code == ResultCode::NxDomain {
                return Err(anyhow::anyhow!("The requested domain {} does not exist", qname.as_ref()));
            }

            // When the --no-recursive option is enabled, we are not
            // looping over the authoritative servers to find an answer.
            // Instead, we are just displaying the latest response.
            if !self.recursive || !recursion_desired {
                return Ok(response);
            }

            // Find authoritative name servers records corresponding to queried domain.
            let mut authoritative_name_servers = Resolver::authoritative_name_servers(&response.authorities);
            // TODO: Loop over all the found servers (instead of using the first one) to maximize
            //       the probability to resolve the queried name.
            let ns = authoritative_name_servers.next();

            if ns.is_none() {
                return Err(anyhow::anyhow!("Recursion not available because no authoritative name servers"));
            }

            let ns = ns.unwrap();
            println!("-- Found Authoritative Name Server: {} -> {}", ns.domain, ns.ns_name);

            // Try to find a valid ip address to use for the selected authoritative name server.
            // It searches in the additional records provided along with the authority records.
            let addr = Resolver::name_server_addr(&ns.ns_name, &response.additionals);
            println!("-- Trying to find A record for {}: {:?}", ns.ns_name, addr);

            server_ip = match addr {
                // For found addresses, resolve the query name with the new authoritative server ip.
                Some(ip) => IpAddr::V4(ip),

                // If the response doesn't contain the name server ip in the additional records section,
                // try to resolve the authoritative name server from the root servers directly.
                None => {
                    let ns_response = self.resolve(&ns.ns_name, QueryType::A, true)?;
                    let ip = ns_response.answers
                        .iter()
                        .find_map(|r| match r {
                            Record::A(protocol::records::A { ip, .. }) => Some(ip),
                            _ => None
                        });

                    match ip {
                        Some(ip) => IpAddr::V4(*ip),
                        None => return Err(anyhow::anyhow!("No recursion available because name server ip not found"))
                    }
                }
            };
        }
    }

    fn authoritative_name_servers(records: &[protocol::packet::Record]) -> impl Iterator<Item=&protocol::records::AuthoritativeNameServer> {
        records
            .iter()
            .filter_map(|r| match r {
                Record::AuthoritativeNameServer(ns) => Some(ns),
                _ => None
            })
    }

    fn name_server_addr(name_server: &str, records: &[protocol::packet::Record]) -> Option<Ipv4Addr> {
        records
            .iter()
            .filter_map(|r| match r {
                Record::A(a) => Some(a),
                _ => None
            })
            .filter(|protocol::records::A { domain, .. }| domain == name_server)
            .map(|protocol::records::A { ip, .. }| *ip)
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
        rand::random()
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
        self.packet.serialize(buf).unwrap();
    }
}