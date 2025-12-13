use common::{DeviceInfo, ConnectionStatus, VideoStreamError, Result};
use dashmap::DashMap;
use quinn::Connection;
use std::sync::Arc;
use std::time::SystemTime;
use tracing::{info, warn};

#[derive(Clone)]
pub struct DeviceManager {
    devices: Arc<DashMap<String, DeviceInfo>>,
    connections: Arc<DashMap<String, Connection>>,
}

impl DeviceManager {
    pub fn new() -> Self {
        Self {
            devices: Arc::new(DashMap::new()),
            connections: Arc::new(DashMap::new()),
        }
    }

    /// 注册设备
    pub fn register_device(&self, device: DeviceInfo) -> Result<()> {
        info!("Registering device: {}", device.device_id);
        self.devices.insert(device.device_id.clone(), device);
        Ok(())
    }

    /// 保存设备连接
    pub fn store_connection(&self, device_id: String, connection: Connection) {
        self.connections.insert(device_id, connection);
    }

    /// 获取设备连接
    pub fn get_connection(&self, device_id: &str) -> Option<Connection> {
        self.connections.get(device_id).map(|c| c.value().clone())
    }

    /// 注销设备
    pub fn unregister_device(&self, device_id: &str) -> Result<()> {
        info!("Unregistering device: {}", device_id);
        self.devices.remove(device_id);
        Ok(())
    }

    /// 获取设备信息
    pub fn get_device(&self, device_id: &str) -> Result<DeviceInfo> {
        self.devices
            .get(device_id)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| VideoStreamError::DeviceNotFound(device_id.to_string()))
    }

    /// 获取所有设备
    pub fn get_all_devices(&self) -> Vec<DeviceInfo> {
        self.devices
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// 获取在线设备
    pub fn get_online_devices(&self) -> Vec<DeviceInfo> {
        self.devices
            .iter()
            .filter(|entry| entry.value().connection_status == ConnectionStatus::Online)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// 更新心跳
    pub fn update_heartbeat(&self, device_id: &str) -> Result<()> {
        if let Some(mut device) = self.devices.get_mut(device_id) {
            device.last_heartbeat = SystemTime::now();
            device.connection_status = ConnectionStatus::Online;
            Ok(())
        } else {
            Err(VideoStreamError::DeviceNotFound(device_id.to_string()))
        }
    }

    /// 设置设备离线
    pub fn set_device_offline(&self, device_id: &str) -> Result<()> {
        if let Some(mut device) = self.devices.get_mut(device_id) {
            warn!("Device {} is now offline", device_id);
            device.connection_status = ConnectionStatus::Offline;
            Ok(())
        } else {
            Err(VideoStreamError::DeviceNotFound(device_id.to_string()))
        }
    }

    /// 检查设备是否在线
    pub fn is_device_online(&self, device_id: &str) -> bool {
        self.devices
            .get(device_id)
            .map(|entry| entry.value().connection_status == ConnectionStatus::Online)
            .unwrap_or(false)
    }
}
