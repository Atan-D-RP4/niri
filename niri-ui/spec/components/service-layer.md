# Service Layer Specification

## Overview

The service layer provides D-Bus integration for system services commonly used by desktop panels and widgets. It wraps complex D-Bus protocols (StatusNotifierItem, FreeDesktop Notifications, MPRIS) in ergonomic Rust and Lua APIs.

**Status**: Specification Draft  
**Priority**: P1 (Phase 2)  
**Dependencies**: zbus, tokio, niri-lua

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Lua API Layer                          │
│  niri.services.tray    niri.services.notifications          │
│  niri.services.mpris   niri.dbus                            │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Service Manager                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │ TrayService │  │ NotifService│  │ MprisService│         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      D-Bus Layer                            │
│  zbus::Connection (session bus)                             │
│  Signal subscriptions, method calls, property watching      │
└─────────────────────────────────────────────────────────────┘
```

## Core Types

### ServiceManager

Central coordinator for all D-Bus services.

```rust
use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use zbus::Connection;

/// Manages lifecycle of all D-Bus services
pub struct ServiceManager {
    connection: Connection,
    tray: Arc<RwLock<TrayService>>,
    notifications: Arc<RwLock<NotificationService>>,
    mpris: Arc<RwLock<MprisService>>,
}

impl ServiceManager {
    /// Create service manager with session bus connection
    pub async fn new() -> zbus::Result<Self> {
        let connection = Connection::session().await?;
        
        Ok(Self {
            connection: connection.clone(),
            tray: Arc::new(RwLock::new(TrayService::new(connection.clone()).await?)),
            notifications: Arc::new(RwLock::new(NotificationService::new(connection.clone()).await?)),
            mpris: Arc::new(RwLock::new(MprisService::new(connection.clone()).await?)),
        })
    }
    
    /// Start all services and begin watching for changes
    pub async fn start(&self) -> zbus::Result<()> {
        self.tray.write().await.start().await?;
        self.notifications.write().await.start().await?;
        self.mpris.write().await.start().await?;
        Ok(())
    }
    
    /// Stop all services
    pub async fn stop(&self) -> zbus::Result<()> {
        self.tray.write().await.stop().await?;
        self.notifications.write().await.stop().await?;
        self.mpris.write().await.stop().await?;
        Ok(())
    }
}
```

### TrayService (StatusNotifierItem/Watcher)

Implements the StatusNotifierWatcher protocol for system tray icons.

```rust
use std::collections::HashMap;

use zbus::Connection;

/// System tray icon information
#[derive(Debug, Clone)]
pub struct TrayItem {
    /// Unique identifier (bus name + object path)
    pub id: String,
    /// Display title
    pub title: String,
    /// Icon name or path
    pub icon_name: Option<String>,
    /// Pixmap data if no icon name
    pub icon_pixmap: Option<IconPixmap>,
    /// Tooltip text
    pub tooltip: Option<String>,
    /// Item category
    pub category: TrayCategory,
    /// Item status
    pub status: TrayStatus,
    /// Whether item has a menu
    pub has_menu: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCategory {
    ApplicationStatus,
    Communications,
    SystemServices,
    Hardware,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayStatus {
    Passive,
    Active,
    NeedsAttention,
}

#[derive(Debug, Clone)]
pub struct IconPixmap {
    pub width: i32,
    pub height: i32,
    pub data: Vec<u8>, // ARGB32
}

/// Events emitted by the tray service
#[derive(Debug, Clone)]
pub enum TrayEvent {
    ItemAdded(TrayItem),
    ItemRemoved { id: String },
    ItemUpdated(TrayItem),
}

pub struct TrayService {
    connection: Connection,
    items: HashMap<String, TrayItem>,
    event_tx: tokio::sync::broadcast::Sender<TrayEvent>,
}

impl TrayService {
    pub async fn new(connection: Connection) -> zbus::Result<Self> {
        let (event_tx, _) = tokio::sync::broadcast::channel(64);
        Ok(Self {
            connection,
            items: HashMap::new(),
            event_tx,
        })
    }
    
    /// Start watching for tray items
    pub async fn start(&mut self) -> zbus::Result<()> {
        // Register as StatusNotifierWatcher on org.kde.StatusNotifierWatcher
        // Watch for RegisterStatusNotifierItem signals
        // Query existing items
        todo!()
    }
    
    /// Get all current tray items
    pub fn items(&self) -> impl Iterator<Item = &TrayItem> {
        self.items.values()
    }
    
    /// Activate a tray item (primary action)
    pub async fn activate(&self, id: &str, x: i32, y: i32) -> zbus::Result<()> {
        // Call Activate method on the item's interface
        todo!()
    }
    
    /// Show context menu for a tray item
    pub async fn context_menu(&self, id: &str, x: i32, y: i32) -> zbus::Result<()> {
        // Call ContextMenu method
        todo!()
    }
    
    /// Secondary activate (middle click)
    pub async fn secondary_activate(&self, id: &str, x: i32, y: i32) -> zbus::Result<()> {
        todo!()
    }
    
    /// Scroll on tray item
    pub async fn scroll(&self, id: &str, delta: i32, orientation: ScrollOrientation) -> zbus::Result<()> {
        todo!()
    }
    
    /// Subscribe to tray events
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<TrayEvent> {
        self.event_tx.subscribe()
    }
    
    pub async fn stop(&mut self) -> zbus::Result<()> {
        // Unregister watcher
        todo!()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ScrollOrientation {
    Horizontal,
    Vertical,
}
```

### NotificationService

Implements org.freedesktop.Notifications for receiving and displaying notifications.

```rust
use std::collections::HashMap;

use zbus::Connection;

/// A notification received from an application
#[derive(Debug, Clone)]
pub struct Notification {
    /// Unique notification ID
    pub id: u32,
    /// Application name
    pub app_name: String,
    /// Application icon name or path
    pub app_icon: Option<String>,
    /// Notification summary/title
    pub summary: String,
    /// Notification body (may contain markup)
    pub body: String,
    /// Available actions as (id, label) pairs
    pub actions: Vec<(String, String)>,
    /// Urgency level
    pub urgency: Urgency,
    /// Timeout in milliseconds (-1 = default, 0 = never)
    pub timeout: i32,
    /// When the notification was received
    pub timestamp: std::time::Instant,
    /// Image data if provided
    pub image: Option<NotificationImage>,
    /// Custom hints
    pub hints: HashMap<String, NotificationHintValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Urgency {
    Low,
    Normal,
    Critical,
}

#[derive(Debug, Clone)]
pub struct NotificationImage {
    pub width: i32,
    pub height: i32,
    pub rowstride: i32,
    pub has_alpha: bool,
    pub bits_per_sample: i32,
    pub channels: i32,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub enum NotificationHintValue {
    String(String),
    Int(i32),
    Bool(bool),
    Byte(u8),
}

/// Events emitted by the notification service
#[derive(Debug, Clone)]
pub enum NotificationEvent {
    /// New notification received
    Received(Notification),
    /// Notification was closed
    Closed { id: u32, reason: CloseReason },
    /// Action was invoked on a notification
    ActionInvoked { id: u32, action_key: String },
}

#[derive(Debug, Clone, Copy)]
pub enum CloseReason {
    Expired,
    Dismissed,
    ClosedByCall,
    Unknown,
}

pub struct NotificationService {
    connection: Connection,
    notifications: HashMap<u32, Notification>,
    event_tx: tokio::sync::broadcast::Sender<NotificationEvent>,
    next_id: u32,
}

impl NotificationService {
    pub async fn new(connection: Connection) -> zbus::Result<Self> {
        let (event_tx, _) = tokio::sync::broadcast::channel(64);
        Ok(Self {
            connection,
            notifications: HashMap::new(),
            event_tx,
            next_id: 1,
        })
    }
    
    /// Start the notification server
    pub async fn start(&mut self) -> zbus::Result<()> {
        // Register on org.freedesktop.Notifications
        // Implement Notify, CloseNotification, GetCapabilities, GetServerInformation
        todo!()
    }
    
    /// Get all active notifications
    pub fn notifications(&self) -> impl Iterator<Item = &Notification> {
        self.notifications.values()
    }
    
    /// Close a notification
    pub async fn close(&mut self, id: u32) -> zbus::Result<()> {
        if self.notifications.remove(&id).is_some() {
            let _ = self.event_tx.send(NotificationEvent::Closed {
                id,
                reason: CloseReason::ClosedByCall,
            });
        }
        Ok(())
    }
    
    /// Invoke an action on a notification
    pub async fn invoke_action(&self, id: u32, action_key: &str) -> zbus::Result<()> {
        let _ = self.event_tx.send(NotificationEvent::ActionInvoked {
            id,
            action_key: action_key.to_string(),
        });
        Ok(())
    }
    
    /// Subscribe to notification events
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<NotificationEvent> {
        self.event_tx.subscribe()
    }
    
    /// Get server capabilities
    pub fn capabilities(&self) -> Vec<&'static str> {
        vec![
            "body",
            "body-markup",
            "body-hyperlinks",
            "body-images",
            "icon-static",
            "actions",
            "persistence",
        ]
    }
    
    pub async fn stop(&mut self) -> zbus::Result<()> {
        todo!()
    }
}
```

### MprisService

Implements MPRIS D-Bus interface for media player control.

```rust
use std::collections::HashMap;

use zbus::Connection;

/// A media player discovered via MPRIS
#[derive(Debug, Clone)]
pub struct MediaPlayer {
    /// Bus name (e.g., "org.mpris.MediaPlayer2.spotify")
    pub bus_name: String,
    /// Player identity/name
    pub identity: String,
    /// Desktop entry name
    pub desktop_entry: Option<String>,
    /// Current playback status
    pub status: PlaybackStatus,
    /// Current track metadata
    pub metadata: TrackMetadata,
    /// Player capabilities
    pub can_play: bool,
    pub can_pause: bool,
    pub can_seek: bool,
    pub can_go_next: bool,
    pub can_go_previous: bool,
    /// Volume (0.0 - 1.0)
    pub volume: f64,
    /// Current position in microseconds
    pub position: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackStatus {
    Playing,
    Paused,
    Stopped,
}

#[derive(Debug, Clone, Default)]
pub struct TrackMetadata {
    pub track_id: Option<String>,
    pub title: Option<String>,
    pub artist: Option<Vec<String>>,
    pub album: Option<String>,
    pub album_artist: Option<Vec<String>>,
    pub art_url: Option<String>,
    pub length: Option<i64>, // microseconds
    pub url: Option<String>,
}

/// Events emitted by the MPRIS service
#[derive(Debug, Clone)]
pub enum MprisEvent {
    PlayerAdded(MediaPlayer),
    PlayerRemoved { bus_name: String },
    PlayerUpdated(MediaPlayer),
    Seeked { bus_name: String, position: i64 },
}

pub struct MprisService {
    connection: Connection,
    players: HashMap<String, MediaPlayer>,
    event_tx: tokio::sync::broadcast::Sender<MprisEvent>,
}

impl MprisService {
    pub async fn new(connection: Connection) -> zbus::Result<Self> {
        let (event_tx, _) = tokio::sync::broadcast::channel(64);
        Ok(Self {
            connection,
            players: HashMap::new(),
            event_tx,
        })
    }
    
    /// Start watching for MPRIS players
    pub async fn start(&mut self) -> zbus::Result<()> {
        // Watch for org.mpris.MediaPlayer2.* names
        // Query properties for each player
        // Subscribe to PropertiesChanged signals
        todo!()
    }
    
    /// Get all discovered players
    pub fn players(&self) -> impl Iterator<Item = &MediaPlayer> {
        self.players.values()
    }
    
    /// Play
    pub async fn play(&self, bus_name: &str) -> zbus::Result<()> {
        todo!()
    }
    
    /// Pause
    pub async fn pause(&self, bus_name: &str) -> zbus::Result<()> {
        todo!()
    }
    
    /// Play/Pause toggle
    pub async fn play_pause(&self, bus_name: &str) -> zbus::Result<()> {
        todo!()
    }
    
    /// Stop
    pub async fn stop(&self, bus_name: &str) -> zbus::Result<()> {
        todo!()
    }
    
    /// Next track
    pub async fn next(&self, bus_name: &str) -> zbus::Result<()> {
        todo!()
    }
    
    /// Previous track
    pub async fn previous(&self, bus_name: &str) -> zbus::Result<()> {
        todo!()
    }
    
    /// Seek to position (microseconds)
    pub async fn seek(&self, bus_name: &str, offset: i64) -> zbus::Result<()> {
        todo!()
    }
    
    /// Set position (microseconds)
    pub async fn set_position(&self, bus_name: &str, track_id: &str, position: i64) -> zbus::Result<()> {
        todo!()
    }
    
    /// Set volume
    pub async fn set_volume(&self, bus_name: &str, volume: f64) -> zbus::Result<()> {
        todo!()
    }
    
    /// Subscribe to MPRIS events
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<MprisEvent> {
        self.event_tx.subscribe()
    }
    
    pub async fn stop(&mut self) -> zbus::Result<()> {
        todo!()
    }
}
```

### Generic D-Bus API

Low-level D-Bus access for custom integrations.

```rust
use zbus::Connection;
use zbus::zvariant::Value;

/// Generic D-Bus method call
pub struct DbusCall {
    pub destination: String,
    pub path: String,
    pub interface: String,
    pub method: String,
    pub args: Vec<Value<'static>>,
}

/// Generic D-Bus property access
pub struct DbusProperty {
    pub destination: String,
    pub path: String,
    pub interface: String,
    pub property: String,
}

pub struct DbusManager {
    connection: Connection,
}

impl DbusManager {
    pub async fn new() -> zbus::Result<Self> {
        Ok(Self {
            connection: Connection::session().await?,
        })
    }
    
    /// Call a D-Bus method
    pub async fn call(&self, call: DbusCall) -> zbus::Result<Value<'static>> {
        todo!()
    }
    
    /// Get a D-Bus property
    pub async fn get_property(&self, prop: DbusProperty) -> zbus::Result<Value<'static>> {
        todo!()
    }
    
    /// Set a D-Bus property
    pub async fn set_property(&self, prop: DbusProperty, value: Value<'_>) -> zbus::Result<()> {
        todo!()
    }
    
    /// Subscribe to a D-Bus signal
    pub async fn subscribe_signal(
        &self,
        sender: Option<&str>,
        path: Option<&str>,
        interface: Option<&str>,
        member: Option<&str>,
    ) -> zbus::Result<tokio::sync::mpsc::Receiver<zbus::Message>> {
        todo!()
    }
}
```

## Lua API

### niri.services.tray

```lua
-- Get all tray items
local items = niri.services.tray.items()
for _, item in ipairs(items) do
    print(item.id, item.title, item.status)
end

-- Subscribe to tray events
niri.services.tray.on_item_added(function(item)
    print("New tray item:", item.title)
end)

niri.services.tray.on_item_removed(function(id)
    print("Removed:", id)
end)

niri.services.tray.on_item_updated(function(item)
    print("Updated:", item.title)
end)

-- Interact with items
niri.services.tray.activate(item.id, x, y)
niri.services.tray.context_menu(item.id, x, y)
niri.services.tray.secondary_activate(item.id, x, y)
niri.services.tray.scroll(item.id, delta, "vertical") -- or "horizontal"
```

### niri.services.notifications

```lua
-- Get active notifications
local notifs = niri.services.notifications.list()

-- Subscribe to events
niri.services.notifications.on_received(function(notif)
    print("Notification:", notif.summary)
    print("  From:", notif.app_name)
    print("  Body:", notif.body)
    print("  Urgency:", notif.urgency)
    
    -- Show actions
    for _, action in ipairs(notif.actions) do
        print("  Action:", action.id, action.label)
    end
end)

niri.services.notifications.on_closed(function(id, reason)
    print("Closed:", id, reason)
end)

-- Interact with notifications
niri.services.notifications.close(id)
niri.services.notifications.invoke_action(id, "action_key")

-- Clear all
niri.services.notifications.clear_all()
```

### niri.services.mpris

```lua
-- Get all players
local players = niri.services.mpris.players()
for _, player in ipairs(players) do
    print(player.identity, player.status)
    if player.metadata.title then
        print("  Playing:", player.metadata.title)
        print("  By:", table.concat(player.metadata.artist or {}, ", "))
    end
end

-- Subscribe to events
niri.services.mpris.on_player_added(function(player)
    print("New player:", player.identity)
end)

niri.services.mpris.on_player_updated(function(player)
    print("Updated:", player.identity, player.status)
end)

-- Control playback
local player = players[1]
if player then
    niri.services.mpris.play(player.bus_name)
    niri.services.mpris.pause(player.bus_name)
    niri.services.mpris.play_pause(player.bus_name)
    niri.services.mpris.next(player.bus_name)
    niri.services.mpris.previous(player.bus_name)
    niri.services.mpris.set_volume(player.bus_name, 0.5)
end
```

### niri.dbus (Generic)

```lua
-- Call a D-Bus method
local result = niri.dbus.call({
    destination = "org.freedesktop.UPower",
    path = "/org/freedesktop/UPower",
    interface = "org.freedesktop.UPower",
    method = "EnumerateDevices",
})

-- Get a property
local percentage = niri.dbus.get_property({
    destination = "org.freedesktop.UPower",
    path = "/org/freedesktop/UPower/devices/battery_BAT0",
    interface = "org.freedesktop.UPower.Device",
    property = "Percentage",
})

-- Set a property
niri.dbus.set_property({
    destination = "org.example.Service",
    path = "/org/example/Object",
    interface = "org.example.Interface",
    property = "SomeProperty",
}, "new_value")

-- Subscribe to signals
niri.dbus.on_signal({
    sender = "org.freedesktop.UPower",
    interface = "org.freedesktop.DBus.Properties",
    member = "PropertiesChanged",
}, function(msg)
    print("Properties changed:", msg)
end)
```

## Acceptance Criteria

### AC-SL-1: Tray Service Discovery

```
GIVEN a StatusNotifierItem application is running
WHEN the tray service starts
THEN the application's tray item appears in items()
AND the item has correct title, icon, and status
```

### AC-SL-2: Tray Item Interaction

```
GIVEN a tray item exists
WHEN activate() is called with coordinates
THEN the item's Activate D-Bus method is invoked
AND the coordinates are passed correctly
```

### AC-SL-3: Notification Reception

```
GIVEN the notification service is running
WHEN an application sends a notification via D-Bus
THEN on_received callback fires with notification data
AND the notification appears in list()
AND id, summary, body, urgency are correctly parsed
```

### AC-SL-4: Notification Actions

```
GIVEN a notification with actions exists
WHEN invoke_action() is called
THEN ActionInvoked signal is emitted on D-Bus
AND on_closed callback fires if action closes notification
```

### AC-SL-5: MPRIS Player Discovery

```
GIVEN a media player with MPRIS support is running
WHEN the MPRIS service starts
THEN the player appears in players()
AND identity, status, metadata are correctly populated
```

### AC-SL-6: MPRIS Playback Control

```
GIVEN an MPRIS player is discovered
WHEN play_pause() is called
THEN the player's PlayPause D-Bus method is invoked
AND player.status updates accordingly
```

### AC-SL-7: Generic D-Bus Call

```
GIVEN valid D-Bus destination, path, interface, method
WHEN niri.dbus.call() is invoked
THEN the method is called on the correct D-Bus interface
AND return value is converted to Lua type
AND errors are propagated as Lua errors
```

### AC-SL-8: Service Graceful Degradation

```
GIVEN a D-Bus service becomes unavailable
WHEN operations are attempted
THEN errors are returned (not compositor crash)
AND service recovers when D-Bus service returns
```

## Test Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tray_item_parsing() {
        // Test parsing of TrayItem from D-Bus properties
    }
    
    #[test]
    fn test_notification_parsing() {
        // Test parsing notification from Notify call arguments
    }
    
    #[test]
    fn test_track_metadata_parsing() {
        // Test parsing MPRIS metadata dictionary
    }
    
    #[test]
    fn test_urgency_conversion() {
        assert_eq!(Urgency::from_byte(0), Urgency::Low);
        assert_eq!(Urgency::from_byte(1), Urgency::Normal);
        assert_eq!(Urgency::from_byte(2), Urgency::Critical);
    }
}
```

### Integration Tests (with mock D-Bus)

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use zbus::blocking::Connection;
    
    // Use dbus-test-runner or mock D-Bus session
    
    #[tokio::test]
    async fn test_notification_roundtrip() {
        // Start notification service
        // Send notification via D-Bus
        // Verify received callback fires
        // Verify notification in list
        // Close notification
        // Verify closed callback fires
    }
    
    #[tokio::test]
    async fn test_tray_item_lifecycle() {
        // Register mock SNI application
        // Verify item_added event
        // Update item properties
        // Verify item_updated event
        // Unregister item
        // Verify item_removed event
    }
}
```

### Lua Integration Tests

```lua
-- test_services.lua
local function test_tray_api()
    -- Verify API exists
    assert(niri.services.tray)
    assert(type(niri.services.tray.items) == "function")
    assert(type(niri.services.tray.activate) == "function")
    assert(type(niri.services.tray.on_item_added) == "function")
end

local function test_notification_api()
    assert(niri.services.notifications)
    assert(type(niri.services.notifications.list) == "function")
    assert(type(niri.services.notifications.close) == "function")
    assert(type(niri.services.notifications.on_received) == "function")
end

local function test_dbus_api()
    assert(niri.dbus)
    assert(type(niri.dbus.call) == "function")
    assert(type(niri.dbus.get_property) == "function")
end
```

## Error Handling

All service methods return `Result` types. Lua bindings convert errors to Lua errors:

```lua
local ok, err = pcall(function()
    niri.services.tray.activate("nonexistent", 0, 0)
end)
if not ok then
    print("Error:", err)
end
```

Services must never panic or crash the compositor (Constraint C1). All D-Bus errors are caught and converted to appropriate error types.

## Performance Considerations

1. **Event Batching**: High-frequency property changes are debounced
2. **Lazy Loading**: Services only connect when first accessed
3. **Connection Sharing**: Single D-Bus connection shared across services
4. **Async Operations**: All D-Bus calls are async, never blocking compositor

## Related Specifications

- [lua_bindings.md](lua_bindings.md) - Lua binding patterns
- [widget.md](widget.md) - Widgets that consume service data
- [compositor-integration.md](compositor-integration.md) - How services integrate with compositor lifecycle
