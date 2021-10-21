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
/// .                    NSDNAME                    .
/// .                                               .
/// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
/// ```

use std::time::Duration;

use crate::result::Result;
use crate::seek::Seek;
use crate::ser::{Serialize, Serializer};

#[derive(Debug)]
pub struct AuthoritativeNameServer {
    pub domain: String,
    pub _class: u16,
    pub ttl: Duration,
    pub ns_name: String,
}

impl Serialize for AuthoritativeNameServer {
    fn serialize<S>(&self, serializer: &mut S) -> Result<()>
        where
            S: Serializer + Seek
    {
        // Name.
        serializer.serialize_qname(&self.domain)?;

        // Type. (Always 2 for NS)
        // See: https://datatracker.ietf.org/doc/html/rfc1035#section-3.2.2
        serializer.serialize_u16(2)?;

        // Class. (IN for now)
        // TODO: Support other class types.
        serializer.serialize_u16(1)?;

        // TTL.
        serializer.serialize_u32(self.ttl.as_secs() as u32)?;

        // Domain name size.
        // Saving a pointer to this field to be able to
        // set the size after domain length computation.
        let size_pos = serializer.position();
        serializer.serialize_u16(0)?;

        // Domain name.
        serializer.serialize_qname(&self.ns_name)?;

        // Domain name serialization length computation and
        // overriding length value.
        let payload_size = serializer.position() - (size_pos + 2);
        let current_position = serializer.position();
        serializer.seek(size_pos)?;
        serializer.serialize_u16(payload_size as u16)?;
        serializer.seek(current_position)
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use crate::byte_packet_buffer::BytePacketBuffer;
    use crate::records::AuthoritativeNameServer;
    use crate::ser::Serialize;

    #[test]
    fn serialize() {
        let mut serializer = BytePacketBuffer::new();
        let ns = AuthoritativeNameServer {
            domain: "test.www.google.com".to_string(),
            _class: 1,
            ttl: Duration::from_secs(60),
            ns_name: "www.google.com".to_string(),
        };

        let res = ns.serialize(&mut serializer);
        assert!(res.is_ok());

        assert_eq!(&[
            0x04, 0x74, 0x65, 0x73, 0x74, // len=4 label="test"
            0x03, 0x77, 0x77, 0x77, // len=3 label="www"
            0x06, 0x67, 0x6f, 0x6f, 0x67, 0x6c, 0x65, // len=6 label="google"
            0x03, 0x63, 0x6f, 0x6d, // len=3 label="com"
            0x00,
            0x00, 0x02, // Type NS.
            0x00, 0x01, // Class IN.
            0x00, 0x00, 0x00, 0x3C, // TTL.
            0x00, 0x10, // RD length.
            0x03, 0x77, 0x77, 0x77, // len=3 label="www"
            0x06, 0x67, 0x6f, 0x6f, 0x67, 0x6c, 0x65, // len=6 label="google"
            0x03, 0x63, 0x6f, 0x6d, // len=3 label="com"
            0x00,
        ], serializer.bytes().as_slice());
    }
}