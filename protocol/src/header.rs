use crate::byte_packet_buffer::BytePacketBuffer;
use crate::result::Result;
use crate::seek::Seek;
use crate::ser::{Serialize, Serializer};

#[derive(Debug, PartialEq)]
pub struct Header {
    pub id: u16,
    pub is_response: bool,
    pub opcode: OpCode,
    pub authoritative_answer: bool,
    pub truncated: bool,
    pub recursion_desired: bool,
    pub recursion_available: bool,
    pub z: bool,
    pub authenticated_data: bool,
    pub checking_disabled: bool,
    pub result_code: ResultCode,
    pub total_questions: u16,
    pub total_answer_records: u16,
    pub total_authority_records: u16,
    pub total_additional_records: u16,
}

impl Default for Header {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum OpCode {
    Query,
    IQuery,
    Status,
}

impl OpCode {
    pub fn from_u8(num: u8) -> OpCode {
        match num {
            1 => OpCode::IQuery,
            2 => OpCode::Status,
            _ => OpCode::Query
        }
    }

    pub fn as_u8(&self) -> u8 {
        match *self {
            OpCode::Query => 0,
            OpCode::IQuery => 1,
            OpCode::Status => 2,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ResultCode {
    NoError,
    FormError,
    ServerFailure,
    NxDomain,
    NotImplemented,
    Refused,
}

impl ResultCode {
    pub fn from_u8(num: u8) -> ResultCode {
        match num {
            1 => ResultCode::FormError,
            2 => ResultCode::ServerFailure,
            3 => ResultCode::NxDomain,
            4 => ResultCode::NotImplemented,
            5 => ResultCode::Refused,
            _ => ResultCode::NoError,
        }
    }

    pub fn as_u8(&self) -> u8 {
        match *self {
            ResultCode::NoError => 0,
            ResultCode::FormError => 1,
            ResultCode::ServerFailure => 2,
            ResultCode::NxDomain => 3,
            ResultCode::NotImplemented => 4,
            ResultCode::Refused => 5,
        }
    }
}

impl Header {
    pub fn new() -> Header {
        Header {
            id: 0,
            is_response: false,
            opcode: OpCode::Query,
            authoritative_answer: false,
            truncated: false,
            recursion_desired: false,
            recursion_available: false,
            z: false,
            authenticated_data: false,
            checking_disabled: false,
            result_code: ResultCode::NoError,
            total_questions: 0,
            total_answer_records: 0,
            total_authority_records: 0,
            total_additional_records: 0,
        }
    }

    pub fn from_buffer(buf: &mut BytePacketBuffer) -> Header {
        let mut header = Header::new();
        header.id = buf.read_u16();

        let byte = buf.read_u8();
        header.is_response = byte >> 7 > 0;
        header.opcode = OpCode::from_u8((byte >> 3) & 0x0F);
        header.authoritative_answer = byte & (1 << 2) > 0;
        header.truncated = byte & (1 << 1) > 0;
        header.recursion_desired = byte & 1 > 0;

        let byte = buf.read_u8();
        header.recursion_available = byte >> 7 > 0;
        header.z = byte & (1 << 6) > 0;
        header.authenticated_data = byte & (1 << 5) > 0;
        header.checking_disabled = byte & (1 << 4) > 0;
        header.result_code = ResultCode::from_u8(byte & 0x0F);

        header.total_questions = buf.read_u16();
        header.total_answer_records = buf.read_u16();
        header.total_authority_records = buf.read_u16();
        header.total_additional_records = buf.read_u16();

        header
    }
}

impl Serialize for Header {
    fn serialize<S>(&self, serializer: &mut S) -> Result<()>
        where
            S: Serializer + Seek
    {
        serializer.serialize_u16(self.id)?;

        let mut byte = 0;
        byte |= self.recursion_desired as u8;
        byte |= (self.truncated as u8) << 1;
        byte |= (self.authoritative_answer as u8) << 2;
        byte |= self.opcode.as_u8() << 3;
        byte |= (self.is_response as u8) << 7;
        serializer.serialize_u8(byte)?;

        byte = self.result_code.as_u8();
        byte |= (self.checking_disabled as u8) << 4;
        byte |= (self.authenticated_data as u8) << 5;
        byte |= (self.z as u8) << 6;
        byte |= (self.recursion_available as u8) << 7;
        serializer.serialize_u8(byte)?;

        serializer.serialize_u16(self.total_questions)?;
        serializer.serialize_u16(self.total_answer_records)?;
        serializer.serialize_u16(self.total_authority_records)?;
        serializer.serialize_u16(self.total_additional_records)
    }
}

#[cfg(test)]
mod test {
    use crate::byte_packet_buffer::BytePacketBuffer;
    use crate::header::{Header, OpCode, ResultCode};
    use crate::ser::Serialize;

    #[test]
    fn parse_header() {
        let packet = &[
            0x5a, 0x3b, 0x01, 0x20, 0x00, 0x01, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x06, 0x67, 0x6f, 0x6f,
            0x67, 0x6c, 0x65, 0x03, 0x63, 0x6f, 0x6d, 0x00,
            0x00, 0x01, 0x00, 0x01
        ];

        let mut buffer = BytePacketBuffer::from_raw_data(packet);
        let header = Header::from_buffer(&mut buffer);

        assert_eq!(Header {
            id: 23099,
            is_response: false,
            opcode: OpCode::Query,
            authoritative_answer: false,
            truncated: false,
            recursion_desired: true,
            recursion_available: false,
            z: false,
            authenticated_data: true,
            checking_disabled: false,
            result_code: ResultCode::NoError,
            total_questions: 1,
            total_answer_records: 0,
            total_authority_records: 0,
            total_additional_records: 0,
        }, header);
    }

    #[test]
    fn serialize_header() {
        let header = Header {
            id: 23099,
            is_response: false,
            opcode: OpCode::Query,
            authoritative_answer: false,
            truncated: false,
            recursion_desired: true,
            recursion_available: false,
            z: false,
            authenticated_data: true,
            checking_disabled: false,
            result_code: ResultCode::NoError,
            total_questions: 1,
            total_answer_records: 0,
            total_authority_records: 0,
            total_additional_records: 0,
        };

        let mut serializer = BytePacketBuffer::new();

        let res = header.serialize(&mut serializer);
        assert!(res.is_ok());

        assert_eq!(&[
            0x5a, 0x3b, 0x01, 0x20, 0x00, 0x01, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00
        ], serializer.bytes().as_slice());
    }
}
