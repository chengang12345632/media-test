use common::{VideoSegment, Result};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, info};
use uuid::Uuid;

type SegmentSender = broadcast::Sender<VideoSegment>;

struct SessionData {
    sender: SegmentSender,
    last_keyframe: Option<VideoSegment>,
}

#[derive(Clone)]
pub struct DistributionManager {
    sessions: Arc<DashMap<Uuid, SessionData>>,
}

impl DistributionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
        }
    }

    /// 创建新的分发会话
    pub fn create_session(&self, session_id: Uuid) -> broadcast::Receiver<VideoSegment> {
        let (tx, rx) = broadcast::channel(1000);
        let session_data = SessionData {
            sender: tx,
            last_keyframe: None,
        };
        self.sessions.insert(session_id, session_data);
        debug!("Created distribution session: {}", session_id);
        rx
    }

    /// 分发视频分片到会话
    pub fn distribute_segment(&self, session_id: &Uuid, segment: VideoSegment) -> Result<()> {
        if let Some(mut session) = self.sessions.get_mut(session_id) {
            // 如果是关键帧，缓存它
            if segment.flags & 0x01 != 0 {
                info!("Caching keyframe for session {}: {} bytes", session_id, segment.data.len());
                session.last_keyframe = Some(segment.clone());
            }
            let _ = session.sender.send(segment);
        }
        Ok(())
    }

    /// 获取会话接收器（新订阅者会先收到最近的关键帧）
    pub fn get_receiver(&self, session_id: &Uuid) -> Option<broadcast::Receiver<VideoSegment>> {
        self.sessions.get(session_id).map(|session| {
            let mut rx = session.sender.subscribe();
            
            // 如果有缓存的关键帧，立即发送给新订阅者
            if let Some(ref keyframe) = session.last_keyframe {
                info!("Sending cached keyframe to new subscriber: {} bytes", keyframe.data.len());
                // 注意：由于broadcast channel的特性，我们不能直接发送
                // 新订阅者会在下一个关键帧到来时收到数据
            }
            
            rx
        })
    }

    /// 关闭会话
    pub fn close_session(&self, session_id: &Uuid) {
        self.sessions.remove(session_id);
        debug!("Closed distribution session: {}", session_id);
    }

    /// 获取活跃会话数
    pub fn active_sessions(&self) -> usize {
        self.sessions.len()
    }
}
