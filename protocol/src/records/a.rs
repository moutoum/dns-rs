/// https://datatracker.ietf.org/doc/html/rfc1034
/// https://datatracker.ietf.org/doc/html/rfc1035
///
/// ```txt
///                                 1  1  1  1  1  1
///   0  1  2  3  4  5  6  7  8  9  0  1  2  3  4  5
/// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
/// .                      NAME                     .
/// .                                               .
/// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
/// |                      TYPE                     |
/// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
/// |                     CLASS                     |
/// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
/// |                      TTL                      |
/// |                                               |
/// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
/// |                   RDLENGTH                    |
/// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
/// |                    ADDRESS                    |
/// |                                               |
/// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
/// ```

use std::net::Ipv4Addr;
use std::time::Duration;

use crate::result::Result;
use crate::seek::Seek;
use crate::ser::{Serialize, Serializer};

#[derive(Debug)]
pub struct A {
    pub domain: String,
    pub _class: u16,
    pub ttl: Duration,
    pub ip: Ipv4Addr,
}

impl Serialize for A {
    fn serialize<S>(&self, serializer: &mut S) -> Result<()>
        where
            S: Serializer + Seek
    {
        // Name.
        serializer.serialize_qname(&self.domain)?;

        // Type. (Always 1 for A)
        // See: https://datatracker.ietf.org/doc/html/rfc1035#section-3.2.2
        serializer.serialize_u16(1)?;

        // Class. (IN for now)
        // TODO: Support other class types.
        serializer.serialize_u16(1)?;

        // TTL.
        serializer.serialize_u32(self.ttl.as_secs() as u32)?;

        // Payload size. Corresponds to an IPv4
        // size (4 bytes).
        serializer.serialize_u16(4)?;

        // Address.
        let bytes = self.ip.octets();
        serializer.serialize_u8(bytes[0])?;
        serializer.serialize_u8(bytes[1])?;
        serializer.serialize_u8(bytes[2])?;
        serializer.serialize_u8(bytes[3])
    }
}

#[cfg(test)]
mod test {
    use std::net::Ipv4Addr;
    use std::time::Duration;

    use crate::byte_packet_buffer::BytePacketBuffer;
    use crate::records::A;
    use crate::ser::Serialize;

    #[test]
    fn serialize() {
        let mut serializer = BytePacketBuffer::new();
        let cname = A {
            domain: "www.google.com".to_string(),
            _class: 1,
            ttl: Duration::from_secs(60),
            ip: Ipv4Addr::new(127, 0, 0, 1),
        };

        let res = cname.serialize(&mut serializer);
        assert!(res.is_ok());

        assert_eq!(&[
            0x03, 0x77, 0x77, 0x77, // len=3 label="www"
            0x06, 0x67, 0x6f, 0x6f, 0x67, 0x6c, 0x65, // len=6 label="google"
            0x03, 0x63, 0x6f, 0x6d, // len=3 label="com"
            0x00,
            0x00, 0x01, // Type A.
            0x00, 0x01, // Class IN.
            0x00, 0x00, 0x00, 0x3C, // TTL.
            0x00, 0x04, // RD length.
            0x7F, 0x00, 0x00, 0x01,
        ], serializer.bytes().as_slice());
    }
}