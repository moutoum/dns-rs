use std::net::Ipv4Addr;
use std::time::Duration;

use crate::byte_packet_buffer::BytePacketBuffer;
use crate::header::Header;
use crate::packet::Record::Unknown;

type Error = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Packet {
    header: Header,
    questions: Vec<Question>,
    answers: Vec<Record>,
    authorities: Vec<Record>,
    resources: Vec<Record>,
}

impl Packet {
    pub fn new() -> Packet {
        Packet {
            header: Header::new(),
            questions: vec![],
            answers: vec![],
            authorities: vec![],
            resources: vec![],
        }
    }

    pub fn from_buffer(buf: &mut BytePacketBuffer) -> Result<Packet> {
        let mut packet = Packet::new();
        packet.header = Header::from_buffer(buf)?;

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
}

#[derive(Debug)]
pub struct Question {
    name: String,
    qtype: QueryType,
    _class: u16,
}

impl Question {
    fn from_buffer(buf: &mut BytePacketBuffer) -> Result<Question> {
        Ok(Question {
            name: buf.read_qname()?,
            qtype: QueryType::from_u16(buf.read_u16()?),
            _class: buf.read_u16()?,
        })
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
    CanonicalName {
        domain: String,
        _class: u16,
        ttl: Duration,
        alias: String,
    }
}

impl Record {
    fn from_buffer(buf: &mut BytePacketBuffer) -> Result<Record> {
        let domain = buf.read_qname()?;
        let qtype = QueryType::from_u16(buf.read_u16()?);
        let class = buf.read_u16()?;
        let ttl = Duration::from_secs(buf.read_u32()? as u64);
        let len = buf.read_u16()?;

        let record = match qtype {
            QueryType::A => Record::A {
                domain,
                _class: class,
                ttl,
                ip: Ipv4Addr::from(buf.read_u32()?),
            },
            QueryType::CanonicalName => Record::CanonicalName {
                domain,
                _class: class,
                ttl,
                alias: buf.read_qname()?,
            },
            _ => Record::Unknown {
                domain,
                qtype,
                _class: class,
                ttl,
                data: buf.read_n(len as usize)?,
            },
        };

        Ok(record)
    }
}