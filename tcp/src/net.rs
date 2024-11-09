use std::sync::{Arc, Mutex};
use std::time::Duration;

enum DeviceType {
    Null = 0x0000,
    Loopback = 0x0001,
    Ethernet = 0x0002,
}

pub const IFNAMSIZ: usize = 16;
pub const NET_DEVICE_ADDR_LEN: usize = 16;

pub const NET_DEVICE_TYPE_NULL: u16 = 0x0000;
pub const NET_DEVICE_TYPE_LOOPBACK: u16 = 0x0001;
pub const NET_DEVICE_TYPE_ETHERNET: u16 = 0x0002;

pub const NET_IFACE_FAMILY_IP: i32 = 1;
pub const NET_IFACE_FAMILY_IPV6: i32 = 2;

pub const NET_PROTOCOL_TYPE_IP: u16 = 0x0800;
pub const NET_PROTOCOL_TYPE_ARP: u16 = 0x0806;
pub const NET_PROTOCOL_TYPE_IPV6: u16 = 0x86dd;

pub const NET_IRQ_SHARED: u16 = 0x0001;

#[derive(Default)]
pub struct NetIface {
    pub next: Option<Box<NetIface>>,
    pub dev: Option<Arc<Mutex<NetDevice>>>,
    pub family: i32,
}

pub struct NetDeviceOps {
    pub open: fn(&mut NetDevice) -> i32,
    pub close: fn(&mut NetDevice) -> i32,
    pub transmit: fn(&mut NetDevice, u16, &[u8], usize, *const u8) -> i32,
    pub poll: fn(&mut NetDevice) -> i32,
}

pub struct NetDevice {
    pub next: Option<Box<NetDevice>>,
    pub index: u32,
    pub device_type: DeviceType,
    // pub mtu: u16,
    // pub flags: NetDeviceFlag,
    // pub hlen: u16,
    // pub alen: u16,
    // pub addr: [u8; NET_DEVICE_ADDR_LEN],
    // pub peer_or_broadcast: [u8; NET_DEVICE_ADDR_LEN],
    // pub ops: Option<NetDeviceOps>,
    // pub priv_data: Option<Arc<Mutex<dyn std::any::Any + Send + Sync>>>,
}

impl NetDevice {

    pub fn new() -> Self {
        NetDevice {
            next: None,
            index: 0,
            device_type: DeviceType::Null,
        }
    }

    pub register

    pub fn state(&self) -> &str {
        if self.is_up() {
            "up"
        } else {
            "down"
        }
    }
}

pub fn net_device_register(dev: Arc<Mutex<NetDevice>>) -> i32 {
    // 実装は省略
    0
}

pub fn net_device_add_iface(dev: &Arc<Mutex<NetDevice>>, iface: Arc<Mutex<NetIface>>) -> i32 {
    // 実装は省略
    0
}

pub fn net_device_get_iface(
    dev: &Arc<Mutex<NetDevice>>,
    family: i32,
) -> Option<Arc<Mutex<NetIface>>> {
    // 実装は省略
    None
}

pub fn net_device_output(
    dev: &Arc<Mutex<NetDevice>>,
    protocol_type: u16,
    data: &[u8],
    dst: *const u8,
) -> i32 {
    // 実装は省略
    0
}

pub fn net_input_handler(protocol_type: u16, data: &[u8], dev: &Arc<Mutex<NetDevice>>) -> i32 {
    // 実装は省略
    0
}

pub fn net_protocol_register(
    name: &str,
    protocol_type: u16,
    handler: fn(&[u8], &Arc<Mutex<NetDevice>>),
) -> i32 {
    // 実装は省略
    0
}

pub fn net_protocol_name(protocol_type: u16) -> &'static str {
    match protocol_type {
        NET_PROTOCOL_TYPE_IP => "IP",
        NET_PROTOCOL_TYPE_ARP => "ARP",
        NET_PROTOCOL_TYPE_IPV6 => "IPv6",
        _ => "Unknown",
    }
}

pub fn net_timer_register(name: &str, interval: Duration, handler: fn()) -> i32 {
    // 実装は省略
    0
}

pub fn net_event_subscribe(
    handler: fn(arg: &mut dyn std::any::Any),
    arg: &mut dyn std::any::Any,
) -> i32 {
    // 実装は省略
    0
}

pub fn net_interrupt() -> i32 {
    // 実装は省略
    0
}

pub fn net_run() -> i32 {
    // 実装は省略
    0
}

pub fn net_shutdown() {
    // 実装は省略
}

pub fn net_init() -> i32 {
    // 実装は省略
    0
}
