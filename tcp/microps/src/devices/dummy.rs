use tracing::info;

use super::base::{DeviceData, DeviceFlags, DeviceOps};

#[derive(Debug, PartialEq, Clone)]
pub struct DummyDevice {
    pub data: DeviceData,
}

impl DummyDevice {
    pub fn new(name: String) -> Self {
        DummyDevice {
            data: DeviceData {
                name,
                mtu: u16::MAX, // IP データグラムの最大サイズ
                flags: DeviceFlags::Down,
                header_length: 0,  // 存在しないので 0
                address_length: 0, // 存在しないので 0
            },
        }
    }
}

impl DeviceOps for DummyDevice {
    fn open(&mut self) {
        if self.data.flags == DeviceFlags::Up {
            info!("device ({}) is already opened", self.data.name);
            return;
        }
        self.data.flags = DeviceFlags::Up;
        info!("device={}, state=up", self.data.name);
    }

    fn close(&mut self) {
        if self.data.flags == DeviceFlags::Down {
            info!("device ({}) is already closed", self.data.name);
            return;
        }
        self.data.flags = DeviceFlags::Down;
        info!("device={}, state=down", self.data.name);
    }

    fn transmit(&mut self, protocol_type: u16, data: &[u8], length: u16, _dest: &[u8]) {
        if self.data.flags == DeviceFlags::Down {
            info!("device ({}) is not opened", self.data.name);
            return;
        }
        if self.data.mtu < length {
            info!(
                "too big packet: MTU is {}, but data size is {}",
                self.data.mtu, length
            );
            return;
        }
        info!(
            "device={}, protocol_type=0x{:x?}, length={}, data={:?}",
            self.data.name, protocol_type, length, data
        );
        return;
    }
}
