#[derive(Debug, Clone, PartialEq, Copy)]
pub enum DeviceFlags {
    Down = 0x0000,
    Up = 0x0001,
    Loopback = 0x0010,
    Broadcast = 0x0020,
    P2P = 0x0040,
    NEED_ARP = 0x0100,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeviceData {
    pub name: String,
    pub mtu: u16,
    pub flags: DeviceFlags,
    pub header_length: u16,
    pub address_length: u16,
}

pub trait DeviceOps {
    fn open(&mut self);
    fn close(&mut self);
    // TODO: 型定義は今は適当
    fn transmit(&mut self, protocol_type: u16, data: &[u8], length: u16, dest: &[u8]);
}
