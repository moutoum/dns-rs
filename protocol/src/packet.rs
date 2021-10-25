use std::net::Ipv4Addr;
use std::time::Duration;

use crate::byte_packet_buffer::BytePacketBuffer;
use crate::header::Header;
use crate::records;
use crate::result::Result;
use crate::seek::Seek;
use crate::ser::{Serialize, Serializer};

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

    pub fn from_buffer(buf: &mut BytePacketBuffer) -> Packet {
        let mut packet = Packet::new();
        packet.header = Header::from_buffer(buf);

        packet.questions = Vec::with_capacity(packet.header.total_questions as usize);
        for _ in 0..packet.header.total_questions {
            packet.questions.push(Question::from_buffer(buf));
        }

        packet.answers = Vec::with_capacity(packet.header.total_answer_records as usize);
        for _ in 0..packet.header.total_answer_records {
            packet.answers.push(Record::from_buffer(buf));
        }

        packet.authorities = Vec::with_capacity(packet.header.total_authority_records as usize);
        for _ in 0..packet.header.total_authority_records {
            packet.authorities.push(Record::from_buffer(buf));
        }

        packet.additionals = Vec::with_capacity(packet.header.total_additional_records as usize);
        for _ in 0..packet.header.total_additional_records {
            packet.additionals.push(Record::from_buffer(buf));
        }

        packet
    }
}

impl Serialize for Packet {
    fn serialize<S>(&self, serializer: &mut S) -> Result<()>
        where
            S: Serializer + Seek,
    {
        self.header.serialize(serializer)?;

        for question in self.questions.iter() {
            question.serialize(serializer)?;
        }

        for answer in self.answers.iter() {
            answer.serialize(serializer)?;
        }

        for authority in self.authorities.iter() {
            authority.serialize(serializer)?;
        }

        for additional in self.additionals.iter() {
            additional.serialize(serializer)?;
        }

        Ok(())
    }
}

#[derive(Debug, Copy, Clone)]
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
    fn from_buffer(buf: &mut BytePacketBuffer) -> Question {
        Question {
            name: buf.read_qname(),
            qtype: QueryType::from_u16(buf.read_u16()),
            _class: buf.read_u16(),
        }
    }
}

impl Serialize for Question {
    fn serialize<S>(&self, serializer: &mut S) -> Result<()>
        where
            S: Serializer + Seek
    {
        serializer.serialize_qname(&self.name)?;
        serializer.serialize_u16(self.qtype.as_u16())?;
        serializer.serialize_u16(1)
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
    A(records::A),
    AuthoritativeNameServer(records::AuthoritativeNameServer),
    CanonicalName(records::CName),
    MailExchange(records::MailExchange),
}

impl Record {
    fn from_buffer(buf: &mut BytePacketBuffer) -> Record {
        let domain = buf.read_qname();
        let qtype = QueryType::from_u16(buf.read_u16());
        let class = buf.read_u16();
        let ttl = Duration::from_secs(buf.read_u32() as u64);
        let len = buf.read_u16();

        match qtype {
            QueryType::A => Record::A(records::A {
                domain,
                _class: class,
                ttl,
                ip: Ipv4Addr::from(buf.read_u32()),
            }),
            QueryType::AuthoritativeNameServer => Record::AuthoritativeNameServer(records::AuthoritativeNameServer {
                domain,
                _class: class,
                ttl,
                ns_name: buf.read_qname(),
            }),
            QueryType::CanonicalName => Record::CanonicalName(records::CName {
                domain,
                _class: class,
                ttl,
                alias: buf.read_qname(),
            }),
            QueryType::MailExchange => Record::MailExchange(records::MailExchange {
                domain,
                _class: class,
                ttl,
                preference: buf.read_u16(),
                exchange: buf.read_qname(),
            }),
            _ => Record::Unknown {
                domain,
                qtype,
                _class: class,
                ttl,
                data: buf.read_n(len as usize),
            },
        }
    }
}

impl Serialize for Record {
    fn serialize<S>(&self, serializer: &mut S) -> Result<()>
        where
            S: Serializer + Seek,
    {
        match self {
            Record::A(record) => { record.serialize(serializer)?; }
            Record::AuthoritativeNameServer(record) => { record.serialize(serializer)?; }
            Record::CanonicalName(record) => { record.serialize(serializer)?; }
            Record::MailExchange(record) => { record.serialize(serializer)?; }
            _ => {}
        };

        Ok(())
    }
}