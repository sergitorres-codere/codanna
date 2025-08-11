//! Notification broadcasting for MCP servers
//!
//! This module provides a broadcast channel for file change events
//! that can be shared between file watchers and multiple MCP server instances.

use std::path::PathBuf;
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub enum FileChangeEvent {
    FileReindexed { path: PathBuf },
    FileCreated { path: PathBuf },
    FileDeleted { path: PathBuf },
    IndexReloaded, // Entire index was reloaded from disk
}

/// Manages notification broadcasting to multiple MCP server instances
#[derive(Clone)]
pub struct NotificationBroadcaster {
    sender: broadcast::Sender<FileChangeEvent>,
    debug: bool,
}

impl NotificationBroadcaster {
    /// Create a new broadcaster with specified channel capacity
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            debug: false,
        }
    }

    /// Enable debug output
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    /// Send a file change event to all subscribers
    pub fn send(&self, event: FileChangeEvent) {
        match self.sender.send(event.clone()) {
            Ok(count) => {
                if self.debug {
                    eprintln!("DEBUG: Broadcast notification to {count} subscribers: {event:?}");
                }
            }
            Err(_) => {
                // No receivers, this is fine
                if self.debug {
                    eprintln!("DEBUG: No subscribers for notification: {event:?}");
                }
            }
        }
    }

    /// Subscribe to receive notifications
    pub fn subscribe(&self) -> broadcast::Receiver<FileChangeEvent> {
        self.sender.subscribe()
    }
}

/// Extension trait for MCP server to handle notifications
impl super::CodeIntelligenceServer {
    /// Start listening for broadcast notifications and forward them via MCP
    pub async fn start_notification_listener(
        &self,
        mut receiver: broadcast::Receiver<FileChangeEvent>,
        mcp_debug: bool,
    ) {
        use rmcp::model::{
            LoggingLevel, LoggingMessageNotificationParam, ResourceUpdatedNotificationParam,
        };

        if mcp_debug {
            eprintln!("DEBUG: MCP server started listening for file change notifications");
        }

        loop {
            match receiver.recv().await {
                Ok(event) => {
                    if mcp_debug {
                        eprintln!("DEBUG: Received broadcast event: {event:?}");
                    }

                    let peer_guard = self.peer.lock().await;
                    if let Some(peer) = peer_guard.as_ref() {
                        match event {
                            FileChangeEvent::FileReindexed { path } => {
                                let path_str = path.display().to_string();

                                if mcp_debug {
                                    eprintln!("DEBUG: Sending MCP notifications for: {path_str}");
                                }

                                // Send resource updated notification
                                let _ = peer
                                    .notify_resource_updated(ResourceUpdatedNotificationParam {
                                        uri: format!("file://{path_str}"),
                                    })
                                    .await;

                                // Send logging message
                                let _ = peer
                                    .notify_logging_message(LoggingMessageNotificationParam {
                                        level: LoggingLevel::Info,
                                        logger: Some("codanna".to_string()),
                                        data: serde_json::json!({
                                            "action": "re-indexed",
                                            "file": path_str
                                        }),
                                    })
                                    .await;

                                if mcp_debug {
                                    eprintln!("DEBUG: MCP notifications sent for: {path_str}");
                                }
                            }
                            FileChangeEvent::FileCreated { path } => {
                                let _ = peer.notify_resource_list_changed().await;
                                if mcp_debug {
                                    eprintln!(
                                        "DEBUG: Sent resource list changed for new file: {path:?}"
                                    );
                                }
                            }
                            FileChangeEvent::FileDeleted { path } => {
                                let _ = peer.notify_resource_list_changed().await;
                                if mcp_debug {
                                    eprintln!(
                                        "DEBUG: Sent resource list changed for deleted file: {path:?}"
                                    );
                                }
                            }
                            FileChangeEvent::IndexReloaded => {
                                let _ = peer.notify_resource_list_changed().await;
                                if mcp_debug {
                                    eprintln!("DEBUG: Sent resource list changed for index reload");
                                }
                            }
                        }
                    } else if mcp_debug {
                        eprintln!("DEBUG: No peer available yet - notification dropped");
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    if mcp_debug {
                        eprintln!("WARNING: Notification receiver lagged by {n} messages");
                    }
                }
                Err(broadcast::error::RecvError::Closed) => {
                    if mcp_debug {
                        eprintln!("DEBUG: Notification channel closed, stopping listener");
                    }
                    break;
                }
            }
        }
    }
}
