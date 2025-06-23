use crate::buffer::BytePacketBuffer;
use crate::utils::Result;

#[derive(PartialEq, Eq, Debug, Clone, Hash, Copy)]
pub enum QueryType {
    UNKNOWN(u16),
    A,     // 1
    NS,    // 2
    CNAME, // 5
    MX,    // 15
    AAAA,  // 28
}

impl QueryType {
    pub fn to_num(&self) -> u16 {
        match *self {
            QueryType::UNKNOWN(x) => x,
            QueryType::A => 1,
            QueryType::NS => 2,
            QueryType::CNAME => 5,
            QueryType::MX => 15,
            QueryType::AAAA => 28,
        }
    }

    pub fn from_num(num: u16) -> QueryType {
        match num {
            1 => QueryType::A,
            2 => QueryType::NS,
            5 => QueryType::CNAME,
            15 => QueryType::MX,
            28 => QueryType::AAAA,
            _ => QueryType::UNKNOWN(num),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsQuestion {
    pub name: String,
    pub qtype: QueryType,
}

impl DnsQuestion {
    pub fn new(name: String, qtype: QueryType) -> DnsQuestion {
        DnsQuestion {
            name: name,
            qtype: qtype,
        }
    }

    pub fn read(&mut self, buffer: &mut BytePacketBuffer) -> Result<()> {
        // 1. read name
        buffer.read_qname(&mut self.name)?;
        // 2. read type
        self.qtype = QueryType::from_num(buffer.read_u16()?);
        // 3. read class
        let _ = buffer.read_u16()?;

        Ok(())
    }

    pub fn write(&self, buffer: &mut BytePacketBuffer) -> Result<()> {
        buffer.write_qname(&self.name)?;

        let typenum = self.qtype.to_num();
        buffer.write_u16(typenum)?;
        // 基本は常に 1 をセットする
        buffer.write_u16(1)?;

        Ok(())
    }
}
