use crate::byte_packet_buffer::BytePacketBuffer;

type Error = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, Error>;

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

#[derive(Debug, PartialEq)]
pub enum OpCode {
    Query = 0,
    IQuery = 1,
    Status = 2,
}

impl OpCode {
    pub fn from_u8(num: u8) -> OpCode {
        match num {
            1 => OpCode::IQuery,
            2 => OpCode::Status,
            0 | _ => OpCode::Query
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ResultCode {
    NoError = 0,
    FormError = 1,
    ServerFailure = 2,
    NXDomain = 3,
    NotImplemented = 4,
    Refused = 5,
}

impl ResultCode {
    pub fn from_u8(num: u8) -> ResultCode {
        match num {
            1 => ResultCode::FormError,
            2 => ResultCode::ServerFailure,
            3 => ResultCode::NXDomain,
            4 => ResultCode::NotImplemented,
            5 => ResultCode::Refused,
            0 | _ => ResultCode::NoError,
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

    pub fn from_buffer(buf: &mut BytePacketBuffer) -> Result<Header> {
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
        header.total_authority_records = buf.read_u16();

        Ok(header)
    }
}

#[cfg(test)]
mod test {
    use crate::byte_packet_buffer::BytePacketBuffer;
    use crate::header::{Header, OpCode, ResultCode};

    #[test]
    fn parse_header() {
        let packet = &[
            0x5a, 0x3b, 0x01, 0x20, 0x00, 0x01, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x06, 0x67, 0x6f, 0x6f,
            0x67, 0x6c, 0x65, 0x03, 0x63, 0x6f, 0x6d, 0x00,
            0x00, 0x01, 0x00, 0x01
        ];

        let mut buffer = BytePacketBuffer::from_raw_data(packet);
        let header = Header::from_buffer(&mut buffer).expect("expected a valid header but got an error");

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
}
