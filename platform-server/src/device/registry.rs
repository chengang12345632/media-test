use common::DeviceInfo;
use std::collections::HashMap;

pub struct DeviceRegistry {
    devices: HashMap<String, DeviceInfo>,
}

impl DeviceRegistry {
    pub fn new() -> Self {
        Self {
            devices: HashMap::new(),
        }
    }

    pub fn add(&mut self, device: DeviceInfo) {
        self.devices.insert(device.device_id.clone(), device);
    }

    pub fn remove(&mut self, device_id: &str) -> Option<DeviceInfo> {
        self.devices.remove(device_id)
    }

    pub fn get(&self, device_id: &str) -> Option<&DeviceInfo> {
        self.devices.get(device_id)
    }

    pub fn list(&self) -> Vec<&DeviceInfo> {
        self.devices.values().collect()
    }
}
