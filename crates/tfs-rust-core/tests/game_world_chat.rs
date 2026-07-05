//! Chat system unit tests — channel registry operations.
//!
//! C++ reference: `chat.cpp` `ChatChannel` methods; `game.cpp` `Game::playerSay`.

use tfs_rust_core::{
    chat::{ChatChannel, ChatRegistry},
};

#[test]
fn channel_registry_basic() {
    // Test basic channel registry operations
    let mut registry = ChatRegistry::new();

    // Add a normal channel
    let channel = ChatChannel::new(4, "Game-Chat".to_string());
    registry.add_normal_channel(channel);

    // Retrieve it
    let retrieved = registry.get_channel(4);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name, "Game-Chat");

    // Non-existent channel
    assert!(registry.get_channel(999).is_none());
}

#[test]
fn channel_public_flag() {
    // Test public channel flag
    let mut registry = ChatRegistry::new();

    let mut channel = ChatChannel::new(4, "Game-Chat".to_string());
    channel.public_channel = true;
    registry.add_normal_channel(channel);

    let retrieved = registry.get_channel(4).unwrap();
    assert!(retrieved.public_channel);
}

#[test]
fn channel_mutable_access() {
    // Test mutable channel access
    let mut registry = ChatRegistry::new();

    let channel = ChatChannel::new(4, "Game-Chat".to_string());
    registry.add_normal_channel(channel);

    // Get mutable reference
    if let Some(channel) = registry.get_channel_mut(4) {
        channel.public_channel = true;
    }

    // Verify mutation
    let retrieved = registry.get_channel(4).unwrap();
    assert!(retrieved.public_channel);
}
