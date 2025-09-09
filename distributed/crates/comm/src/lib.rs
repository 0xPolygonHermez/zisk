use chrono::{DateTime, Utc};
use distributed_config::CommConfig;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tracing::{info, instrument, warn};
use uuid::Uuid;

pub type Result<T> = std::result::Result<T, anyhow::Error>;

/// Manages peer-to-peer communication
#[derive(Debug)]
pub struct CommManager {
    config: CommConfig,
    peers: Arc<RwLock<HashMap<Uuid, PeerInfo>>>,
    _discovery_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub id: Uuid,
    pub address: SocketAddr,
    pub last_seen: DateTime<Utc>,
    pub status: PeerStatus,
    pub capabilities: Vec<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PeerStatus {
    Connected,
    Disconnected,
    Connecting,
    Error(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PeerMessage {
    pub id: Uuid,
    pub message_type: MessageType,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub source: Uuid,
    pub target: Option<Uuid>, // None for broadcast
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MessageType {
    Heartbeat,
    TaskRequest,
    TaskResponse,
    PeerDiscovery,
    Custom(String),
}

impl CommManager {
    #[instrument(skip(config))]
    pub async fn new(config: CommConfig) -> Result<Self> {
        info!("Initializing communication manager");

        let peers = Arc::new(RwLock::new(HashMap::new()));

        Ok(Self { config, peers, _discovery_enabled: true })
    }

    #[instrument(skip(self))]
    pub async fn start_discovery(&self) -> Result<()> {
        info!("Starting peer discovery");

        // This is a placeholder for actual peer discovery implementation
        // In a real implementation, this would:
        // 1. Start listening for peer announcements
        // 2. Periodically broadcast our own presence
        // 3. Maintain a routing table of known peers
        // 4. Handle peer connection/disconnection events

        Ok(())
    }

    #[instrument(skip(self, peer_info))]
    pub async fn add_peer(&self, peer_info: PeerInfo) -> Result<()> {
        let mut peers = self.peers.write().await;
        info!("Adding peer: {} at {}", peer_info.id, peer_info.address);
        peers.insert(peer_info.id, peer_info);
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn remove_peer(&self, peer_id: Uuid) -> Result<()> {
        let mut peers = self.peers.write().await;
        if peers.remove(&peer_id).is_some() {
            info!("Removed peer: {}", peer_id);
        } else {
            warn!("Attempted to remove unknown peer: {}", peer_id);
        }
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn get_peers(&self) -> Vec<PeerInfo> {
        let peers = self.peers.read().await;
        peers.values().cloned().collect()
    }

    #[instrument(skip(self))]
    pub async fn get_peer_count(&self) -> usize {
        let peers = self.peers.read().await;
        peers.len()
    }

    #[instrument(skip(self, message))]
    pub async fn send_message(&self, message: PeerMessage) -> Result<()> {
        // Placeholder for actual message sending implementation
        // In a real implementation, this would:
        // 1. Serialize the message
        // 2. Route it to the appropriate peer(s)
        // 3. Handle delivery confirmations
        // 4. Retry on failure

        info!("Sending message {} to peer {:?}", message.id, message.target);
        Ok(())
    }

    #[instrument(skip(self, message))]
    pub async fn broadcast_message(&self, message: PeerMessage) -> Result<()> {
        let peers = self.peers.read().await;
        let peer_count = peers.len();

        info!("Broadcasting message {} to {} peers", message.id, peer_count);

        // Placeholder for actual broadcast implementation
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn heartbeat(&self) -> Result<()> {
        let peers = self.peers.read().await;
        let now = Utc::now();

        // Check for stale peers (haven't been seen recently)
        for (peer_id, peer_info) in peers.iter() {
            let time_since_last_seen = now.signed_duration_since(peer_info.last_seen);

            if time_since_last_seen.num_seconds()
                > self.config.heartbeat_interval_seconds as i64 * 3
            {
                warn!("Peer {} appears stale (last seen: {})", peer_id, peer_info.last_seen);
            }
        }

        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down communication manager");

        // Placeholder for cleanup:
        // 1. Close all active connections
        // 2. Send disconnect messages to peers
        // 3. Stop background tasks

        let mut peers = self.peers.write().await;
        peers.clear();

        Ok(())
    }
}

impl PeerInfo {
    pub fn new(id: Uuid, address: SocketAddr) -> Self {
        Self {
            id,
            address,
            last_seen: Utc::now(),
            status: PeerStatus::Connecting,
            capabilities: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_capabilities(mut self, capabilities: Vec<String>) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn mark_connected(&mut self) {
        self.status = PeerStatus::Connected;
        self.last_seen = Utc::now();
    }

    pub fn mark_disconnected(&mut self) {
        self.status = PeerStatus::Disconnected;
    }

    pub fn update_last_seen(&mut self) {
        self.last_seen = Utc::now();
    }
}

impl PeerMessage {
    pub fn new(message_type: MessageType, payload: serde_json::Value, source: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            message_type,
            payload,
            timestamp: Utc::now(),
            source,
            target: None,
        }
    }

    pub fn with_target(mut self, target: Uuid) -> Self {
        self.target = Some(target);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[tokio::test]
    async fn test_comm_manager_creation() {
        let config = CommConfig {
            max_peers: 10,
            discovery_interval_seconds: 30,
            heartbeat_interval_seconds: 10,
            connection_timeout_seconds: 30,
        };

        let manager = CommManager::new(config).await.unwrap();
        assert_eq!(manager.get_peer_count().await, 0);
    }

    #[tokio::test]
    async fn test_peer_management() {
        let config = CommConfig {
            max_peers: 10,
            discovery_interval_seconds: 30,
            heartbeat_interval_seconds: 10,
            connection_timeout_seconds: 30,
        };

        let manager = CommManager::new(config).await.unwrap();
        let peer_id = Uuid::new_v4();
        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

        let peer_info = PeerInfo::new(peer_id, address);
        manager.add_peer(peer_info).await.unwrap();

        assert_eq!(manager.get_peer_count().await, 1);

        manager.remove_peer(peer_id).await.unwrap();
        assert_eq!(manager.get_peer_count().await, 0);
    }
}
