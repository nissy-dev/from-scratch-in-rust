const NET_DEVICE_ADDR_LEN: usize = 16;

struct NetDevice {
    next_device: Option<Box<NetDevice>>,
    index: u32,
    name: String,
    device_type: u16,
    mtu: u16,
    flags: u16,
    header_length: u16,
    address_length: u16,
    address: [u8; NET_DEVICE_ADDR_LEN],
    net_device_ops: impl NetDeviceOps,
}

trait NetDeviceOps {
    fn open(dev: &NetDevice) -> i32;
    fn close(dev: &NetDevice) -> i32;
    fn open(transmit: &NetDevice, device_type: u16, data: u8, len: usize, dst: Fn) -> i32;
}
