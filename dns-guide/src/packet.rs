use std::net::Ipv4Addr;

use crate::{
    buffer::BytePacketBuffer,
    header::DnsHeader,
    question::{DnsQuestion, QueryType},
    record::DnsRecord,
    utils::Result,
};

#[derive(Clone, Debug)]
pub struct DnsPacket {
    pub header: DnsHeader,
    pub questions: Vec<DnsQuestion>,
    pub answers: Vec<DnsRecord>,
    pub authorities: Vec<DnsRecord>,
    pub resources: Vec<DnsRecord>,
}

impl DnsPacket {
    pub fn new() -> DnsPacket {
        DnsPacket {
            header: DnsHeader::new(),
            questions: Vec::new(),
            answers: Vec::new(),
            authorities: Vec::new(),
            resources: Vec::new(),
        }
    }

    pub fn from_buffer(buffer: &mut BytePacketBuffer) -> Result<DnsPacket> {
        let mut result = DnsPacket::new();
        result.header.read(buffer)?;

        for _ in 0..result.header.questions {
            let mut question = DnsQuestion::new("".to_string(), QueryType::UNKNOWN(0));
            question.read(buffer)?;
            result.questions.push(question);
        }

        for _ in 0..result.header.answers {
            let rec = DnsRecord::read(buffer)?;
            result.answers.push(rec);
        }
        for _ in 0..result.header.authoritative_entries {
            let rec = DnsRecord::read(buffer)?;
            result.authorities.push(rec);
        }
        for _ in 0..result.header.resource_entries {
            let rec = DnsRecord::read(buffer)?;
            result.resources.push(rec);
        }

        Ok(result)
    }

    pub fn write(&mut self, buffer: &mut BytePacketBuffer) -> Result<()> {
        self.header.questions = self.questions.len() as u16;
        self.header.answers = self.answers.len() as u16;
        self.header.authoritative_entries = self.authorities.len() as u16;
        self.header.resource_entries = self.resources.len() as u16;

        self.header.write(buffer)?;

        for question in &self.questions {
            question.write(buffer)?;
        }
        for rec in &self.answers {
            rec.write(buffer)?;
        }
        for rec in &self.authorities {
            rec.write(buffer)?;
        }
        for rec in &self.resources {
            rec.write(buffer)?;
        }

        Ok(())
    }

    pub fn get_random_a(&self) -> Option<Ipv4Addr> {
        self.answers
            .iter()
            .filter_map(|record| match record {
                DnsRecord::A { addr, .. } => Some(*addr),
                _ => None,
            })
            .next()
    }

    fn get_ns<'a>(&'a self, qname: &'a str) -> impl Iterator<Item = (&'a str, &'a str)> {
        self.authorities
            .iter()
            .filter_map(|record| match record {
                DnsRecord::NS { domain, host, .. } => Some((domain.as_str(), host.as_str())),
                _ => None,
            })
            .filter(move |(domain, _)| qname.ends_with(*domain))
    }

    // 次のように NS レコードに対する A レコードを reosurce セクションから取得できる
    //
    // # dig +norecurse @192.5.6.30 www.google.com
    //
    // ; <<>> DiG 9.10.3-P4-Ubuntu <<>> +norecurse @192.5.6.30 www.google.com
    // ; (1 server found)
    // ;; global options: +cmd
    // ;; Got answer:
    // ;; ->>HEADER<<- opcode: QUERY, status: NOERROR, id: 16229
    // ;; flags: qr; QUERY: 1, ANSWER: 0, AUTHORITY: 4, ADDITIONAL: 5
    //
    // ;; OPT PSEUDOSECTION:
    // ; EDNS: version: 0, flags:; udp: 4096
    // ;; QUESTION SECTION:
    // ;www.google.com.			IN	A
    //
    // ;; AUTHORITY SECTION:
    // google.com.		172800	IN	NS	ns2.google.com.
    // google.com.		172800	IN	NS	ns1.google.com.
    // google.com.		172800	IN	NS	ns3.google.com.
    // google.com.		172800	IN	NS	ns4.google.com.

    // ;; ADDITIONAL SECTION:
    // ns2.google.com.		172800	IN	A	216.239.34.10
    // ns1.google.com.		172800	IN	A	216.239.32.10
    // ns3.google.com.		172800	IN	A	216.239.36.10
    // ns4.google.com.		172800	IN	A	216.239.38.10
    //
    pub fn get_resolved_ns(&self, qname: &str) -> Option<Ipv4Addr> {
        self.get_ns(qname)
            .flat_map(|(_, host)| {
                self.resources
                    .iter()
                    .filter_map(move |record| match record {
                        DnsRecord::A { domain, addr, .. } if domain == host => Some(addr),
                        _ => None,
                    })
            })
            .map(|addr| *addr)
            // Finally, pick the first valid entry
            .next()
    }

    // 次の NS server の ipv4 アドレスがわからなかった場合
    pub fn get_unresolved_ns<'a>(&'a self, qname: &'a str) -> Option<&'a str> {
        self.get_ns(qname).map(|(_, host)| host).next()
    }
}
