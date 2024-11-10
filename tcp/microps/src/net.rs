// use tracing::info;

// enum DeviceType {
//     Dummy = 0x0000,
//     Loopback = 0x0001,
//     Ethernet = 0x0002,
// }

// #[derive(Debug, PartialEq)]
// enum DeviceFlags {
//     Down = 0x0000,
//     Up = 0x0001,
//     Loopback = 0x0010,
//     Broadcast = 0x0020,
//     P2P = 0x0040,
//     NEED_ARP = 0x0100,
// }

// struct NetDevice {
//     device_type: DeviceType,
//     name: String,
//     mtu: u16,
//     flags: DeviceFlags,
//     header_length: u16,
//     address_length: u16,
// }

// trait NetDeviceOps {
//     fn open(&self);
//     fn close(&self);
//     fn transmit(&self, protocol_type: u16, data: &[u8], length: u16, dest: );
// }

// impl NetDevice {
//     fn new(device_type: DeviceType, name: String) -> Self {
//         NetDevice {
//             device_type,
//             name,
//             mtu: 1500, // TODO
//             flags: DeviceFlags::Down,
//             header_length: 14, // TODO
//             address_length: 6, // TODO

//         }
//     }

//     // fn open(&mut self) {
//     //     if self.flags == DeviceFlags::Up {
//     //         info!("device ({}) is already opened", self.name);
//     //         return;
//     //     }
//     //     // TODO: open device
//     //     self.flags = DeviceFlags::Up;
//     // }

//     // fn close(&mut self) {
//     //     if self.flags == DeviceFlags::Up {
//     //         info!("device ({}) is already closed", self.name);
//     //         return;
//     //     }
//     //     // TODO: close device
//     //     self.flags = DeviceFlags::Down;
//     // }

//     // fn transmit(&self, protocol_type: data: &[u8]) {

//     // }
// }

// impl NetDeviceOps for NetDevice {
//     fn open(&self) {
//         if self.flags == DeviceFlags::Up {
//             info!("device ({}) is already opened", self.name);
//             return;
//         }
//     }

//     fn close(&self) {
//         if self.flags == DeviceFlags::Up {
//             info!("device ({}) is already closed", self.name);
//             return;
//         }
//     }

//     // fn transmit(&self, protocol_type: u16, data: &[u8], length: u16) {}
// }

// struct NetDeviceList {
//     head: Option<Box<NetDevice>>,
// }

// impl NetDeviceList {
//     fn new() -> Self {
//         NetDeviceList { head: None }
//     }

//     fn register(&mut self, device: NetDevice) {
//         let mut new_device = Box::new(device);
//         new_device.next_device = self.head.take();
//         self.head = Some(new_device);
//     }
// }

// // trait NetDeviceOps {
// //     fn open(&self);
// //     fn close(&self);
// //     fn tran
// // }

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_register() {
//         let mut devices = NetDeviceList::new();
//         let eth0 = NetDevice::new(DeviceType::Ethernet, "eth0".to_string());
//         let eth1 = NetDevice::new(DeviceType::Ethernet, "eth1".to_string());
//         devices.register(eth0);
//         devices.register(eth1);
//         assert_eq!(devices.head.is_some(), true);
//         let head = devices.head.as_ref().unwrap();
//         assert_eq!(head.name, "eth1");
//         assert_eq!(head.next_device.is_some(), true);
//         let next = head.next_device.as_ref().unwrap();
//         assert_eq!(next.name, "eth0");
//         assert_eq!(next.next_device.is_none(), true);
//     }
// }
