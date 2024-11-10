use std::sync::{Arc, Mutex};

use base::{DeviceData, DeviceOps};
use dummy::DummyDevice;
use tracing::info;

mod base;
mod dummy;

pub enum DeviceType {
    Dummy,
    Loopback,
    Ethernet,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Device {
    DummyDevice(DummyDevice),
    LoopbackDevice,
    EthernetDevice,
}

impl Device {
    pub fn new(name: &str, device_type: DeviceType) -> Self {
        let name = name.to_string();
        match device_type {
            DeviceType::Dummy => Device::DummyDevice(DummyDevice::new(name)),
            DeviceType::Loopback => Device::LoopbackDevice,
            DeviceType::Ethernet => Device::EthernetDevice,
        }
    }

    fn data(&self) -> &DeviceData {
        match self {
            Device::DummyDevice(device) => &device.data,
            Device::LoopbackDevice => unimplemented!("todo"),
            Device::EthernetDevice => unimplemented!("todo"),
        }
    }
}

impl DeviceOps for Device {
    fn open(&mut self) {
        match self {
            Device::DummyDevice(device) => device.open(),
            Device::LoopbackDevice => unimplemented!("todo"),
            Device::EthernetDevice => unimplemented!("todo"),
        }
    }

    fn close(&mut self) {
        match self {
            Device::DummyDevice(device) => device.close(),
            Device::LoopbackDevice => unimplemented!("todo"),
            Device::EthernetDevice => unimplemented!("todo"),
        }
    }

    fn transmit(&mut self, protocol_type: u16, data: &[u8], length: u16, dest: &[u8]) {
        match self {
            Device::DummyDevice(device) => device.transmit(protocol_type, data, length, dest),
            Device::LoopbackDevice => unimplemented!("todo"),
            Device::EthernetDevice => unimplemented!("todo"),
        }
    }
}

struct DeviceList<T: DeviceOps + Clone> {
    device: T,
    next: Option<Arc<Mutex<DeviceList<T>>>>,
}

impl<T: DeviceOps + Clone> DeviceList<T> {
    fn new(device: T) -> Self {
        DeviceList { device, next: None }
    }

    fn register(&mut self, device: T) {
        let new_device_list = DeviceList {
            device,
            next: Some(Arc::new(Mutex::new(DeviceList {
                device: self.device.clone(),
                next: self.next.clone(),
            }))),
        };
        *self = new_device_list;
    }

    fn open(&mut self) {
        info!("open all devices...");
        self.device.open();
        let mut current = self.next.clone();
        while let Some(device_list) = current {
            let mut device_list = device_list.lock().unwrap();
            device_list.device.open();
            current = device_list.next.clone();
        }
    }

    fn close(&mut self) {
        info!("close all devices...");
        self.device.close();
        let mut current = self.next.clone();
        while let Some(device_list) = current {
            let mut device_list = device_list.lock().unwrap();
            device_list.device.close();
            current = device_list.next.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{thread::sleep, time::Duration};

    use super::*;

    #[test]
    fn test_register() {
        let dummy0 = Device::new("d0", DeviceType::Dummy);
        let mut list = DeviceList::new(dummy0);
        let dummy1 = Device::new("d1", DeviceType::Dummy);
        list.register(dummy1);

        assert_eq!(list.device.data().name, "d1");
        assert_eq!(list.next.is_some(), true);
        let next = &list.next.unwrap();
        let next = next.lock().unwrap();
        assert_eq!(next.device.data().name, "d0");
        assert_eq!(next.next.is_some(), false);
    }

    #[test]
    fn test_open() {
        let dummy0 = Device::new("d0", DeviceType::Dummy);
        let mut list = DeviceList::new(dummy0);
        let dummy1 = Device::new("d1", DeviceType::Dummy);
        list.register(dummy1);

        assert_eq!(list.device.data().flags, base::DeviceFlags::Down);

        list.open();
        assert_eq!(list.device.data().flags, base::DeviceFlags::Up);
        let next = &list.next.unwrap();
        let next = next.lock().unwrap();
        assert_eq!(next.device.data().flags, base::DeviceFlags::Up);
    }

    #[test]
    fn test_e2e() {
        tracing_subscriber::fmt().init();

        let d0 = Device::new("d0", DeviceType::Dummy);
        let mut list = DeviceList::new(d0);
        list.open();

        for _ in 0..3 {
            list.device.transmit(0x0800, &[0; 0], 0, &[0; 0]);
            sleep(Duration::from_secs(1));
        }

        list.close();
        assert_eq!(list.device.data().flags, base::DeviceFlags::Down);
        return;
    }
}
