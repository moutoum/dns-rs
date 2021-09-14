use std::net::Ipv4Addr;
use std::time::Duration;

use crate::byte_packet_buffer::BytePacketBuffer;
use crate::header::Header;
use crate::packet::Record::Unknown;

type Error = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Packet {
    pub header: Header,
    pub questions: Vec<Question>,
    pub answers: Vec<Record>,
    pub authorities: Vec<Record>,
    pub additionals: Vec<Record>,
}

impl Packet {
    pub fn new() -> Packet {
        Packet {
            header: Header::new(),
            questions: vec![],
            answers: vec![],
            authorities: vec![],
            additionals: vec![],
        }
    }

    pub fn from_buffer(buf: &mut BytePacketBuffer) -> Result<Packet> {
        let mut packet = Packet::new();
        packet.header = Header::from_buffer(buf);

        packet.questions = Vec::with_capacity(packet.header.total_questions as usize);
        for _ in 0..packet.header.total_questions {
            packet.questions.push(Question::from_buffer(buf)?);
        }

        packet.answers = Vec::with_capacity(packet.header.total_answer_records as usize);
        for _ in 0..packet.header.total_answer_records {
            packet.answers.push(Record::from_buffer(buf)?);
        }

        Ok(packet)
    }

    pub fn write_to_buffer(&self, buf: &mut BytePacketBuffer) {
        self.header.write_to_buffer(buf);

        self.questions.iter().for_each(|question| question.write_to_buffer(buf));
        self.answers.iter().for_each(|answer| answer.write_to_buffer(buf));
        self.authorities.iter().for_each(|answer| answer.write_to_buffer(buf));
        self.additionals.iter().for_each(|answer| answer.write_to_buffer(buf));
    }
}

#[derive(Debug)]
pub enum QueryType {
    Unknown(u16),
    // A, IPv4 address.
    A,
    // NS, Authoritative name server.
    AuthoritativeNameServer,
    // MD, Mail destination. Obsolete use MX instead.
    MailDestination,
    // MF, Mail forwarder. Obsolete use MX instead.
    MailForwarder,
    // CNAME, Canonical name for an alias.
    CanonicalName,
    // SOA, Marks the start of a zone of authority.
    StartOfAuthority,
    // MB, Mailbox domain name.
    Mailbox,
    // MG, Mail group member.
    MailGroup,
    // MR, Mail rename domain name.
    MailRename,
    // NULL, Null resource record.
    Null,
    // WKS, Well known service description.
    WellKnownService,
    // PTR, Domain name pointer.
    DomainPointer,
    // HINFO, Host information.
    HostInformation,
    // MINFO, Mailbox or mail list information.
    MailInformation,
    // MX, Mail exchange.
    MailExchange,
    // TXT, Text strings.
    Text,
}

impl QueryType {
    pub fn from_u16(num: u16) -> QueryType {
        match num {
            1 => QueryType::A,
            2 => QueryType::AuthoritativeNameServer,
            3 => QueryType::MailDestination,
            4 => QueryType::MailForwarder,
            5 => QueryType::CanonicalName,
            6 => QueryType::StartOfAuthority,
            7 => QueryType::Mailbox,
            8 => QueryType::MailGroup,
            9 => QueryType::MailRename,
            10 => QueryType::Null,
            11 => QueryType::WellKnownService,
            12 => QueryType::DomainPointer,
            13 => QueryType::HostInformation,
            14 => QueryType::MailInformation,
            15 => QueryType::MailExchange,
            16 => QueryType::Text,
            _ => QueryType::Unknown(num),
        }
    }

    pub fn as_u16(&self) -> u16 {
        match *self {
            QueryType::A => 1,
            QueryType::AuthoritativeNameServer => 2,
            QueryType::MailDestination => 3,
            QueryType::MailForwarder => 4,
            QueryType::CanonicalName => 5,
            QueryType::StartOfAuthority => 6,
            QueryType::Mailbox => 7,
            QueryType::MailGroup => 8,
            QueryType::MailRename => 9,
            QueryType::Null => 10,
            QueryType::WellKnownService => 11,
            QueryType::DomainPointer => 12,
            QueryType::HostInformation => 13,
            QueryType::MailInformation => 14,
            QueryType::MailExchange => 15,
            QueryType::Text => 16,
            QueryType::Unknown(num) => num,
        }
    }
}

#[derive(Debug)]
pub struct Question {
    pub name: String,
    pub qtype: QueryType,
    pub _class: u16,
}

impl Question {
    fn from_buffer(buf: &mut BytePacketBuffer) -> Result<Question> {
        Ok(Question {
            name: buf.read_qname(),
            qtype: QueryType::from_u16(buf.read_u16()),
            _class: buf.read_u16(),
        })
    }

    fn write_to_buffer(&self, buf: &mut BytePacketBuffer) {
        buf.write_qname(&self.name);
        buf.write_u16(self.qtype.as_u16());
        buf.write_u16(1);
    }
}

#[derive(Debug)]
pub enum Record {
    Unknown {
        domain: String,
        qtype: QueryType,
        _class: u16,
        ttl: Duration,
        data: Vec<u8>,
    },
    A {
        domain: String,
        _class: u16,
        ttl: Duration,
        ip: Ipv4Addr,
    },
    AuthoritativeNameServer {
        domain: String,
        _class: u16,
        ttl: Duration,
        ns_name: String,
    },
    CanonicalName {
        domain: String,
        _class: u16,
        ttl: Duration,
        alias: String,
    },
    MailExchange {
        domain: String,
        _class: u16,
        ttl: Duration,
        preference: u16,
        exchange: String,
    },
}

impl Record {
    fn from_buffer(buf: &mut BytePacketBuffer) -> Result<Record> {
        let domain = buf.read_qname();
        let qtype = QueryType::from_u16(buf.read_u16());
        let class = buf.read_u16();
        let ttl = Duration::from_secs(buf.read_u32() as u64);
        let len = buf.read_u16();

        let record = match qtype {
            QueryType::A => Record::A {
                domain,
                _class: class,
                ttl,
                ip: Ipv4Addr::from(buf.read_u32()),
            },
            QueryType::AuthoritativeNameServer => Record::AuthoritativeNameServer {
                domain,
                _class: class,
                ttl,
                ns_name: buf.read_qname(),
            },
            QueryType::CanonicalName => Record::CanonicalName {
                domain,
                _class: class,
                ttl,
                alias: buf.read_qname(),
            },
            QueryType::MailExchange => Record::MailExchange {
                domain,
                _class: class,
                ttl,
                preference: buf.read_u16(),
                exchange: buf.read_qname(),
            },
            _ => Record::Unknown {
                domain,
                qtype,
                _class: class,
                ttl,
                data: buf.read_n(len as usize),
            },
        };

        Ok(record)
    }

    fn write_to_buffer(&self, buf: &mut BytePacketBuffer) {
        match self {
            Record::A { domain, ttl, ip, .. } => {
                buf.write_qname(&domain);
                buf.write_u16(QueryType::A.as_u16());
                buf.write_u16(1);
                buf.write_u32(ttl.as_secs() as u32);
                buf.write_u16(4);
                let bytes = ip.octets();
                buf.write_u8(bytes[0]);
                buf.write_u8(bytes[1]);
                buf.write_u8(bytes[2]);
                buf.write_u8(bytes[3]);
            },
            Record::AuthoritativeNameServer { domain, _class, ttl, ns_name } => {
                buf.write_qname(&domain);
                buf.write_u16(QueryType::AuthoritativeNameServer.as_u16());
                buf.write_u16(1);
                buf.write_u32(ttl.as_secs() as u32);
                let size_pos = buf.pos();
                buf.write_u16(0);
                buf.write_qname(ns_name);
                let payload_size = buf.pos() - size_pos + 2;
                buf.set_u16(size_pos,  payload_size as u16);
            },
            Record::CanonicalName { domain, _class, ttl, alias } => {
                buf.write_qname(&domain);
                buf.write_u16(QueryType::AuthoritativeNameServer.as_u16());
                buf.write_u16(1);
                buf.write_u32(ttl.as_secs() as u32);
                let size_pos = buf.pos();
                buf.write_u16(0);
                buf.write_qname(alias);
                let payload_size = buf.pos() - size_pos + 2;
                buf.set_u16(size_pos,  payload_size as u16);
            }
            Record::MailExchange { domain, _class, ttl, preference, exchange } => {
                buf.write_qname(&domain);
                buf.write_u16(QueryType::AuthoritativeNameServer.as_u16());
                buf.write_u16(1);
                buf.write_u32(ttl.as_secs() as u32);
                let size_pos = buf.pos();
                buf.write_u16(0);
                buf.write_u16(*preference);
                buf.write_qname(exchange);
                let payload_size = buf.pos() - size_pos + 2;
                buf.set_u16(size_pos,  payload_size as u16);
            }
            _ => {},
        };
    }
}