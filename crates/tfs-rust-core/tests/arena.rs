use slotmap::SlotMap;
use tfs_rust_common::Position;
use tfs_rust_core::{CreatureId, CreatureKind, PlayerStub};

#[test]
fn slotmap_generation_invalidation() {
    let mut sm: SlotMap<CreatureId, CreatureKind> = SlotMap::with_key();
    let id = sm.insert(CreatureKind::Player(PlayerStub {
        name: "a".to_string(),
        guid: 1,
        position: Position::new(1, 1, 7),
    }));
    sm.remove(id);
    let id2 = sm.insert(CreatureKind::Player(PlayerStub {
        name: "b".to_string(),
        guid: 2,
        position: Position::new(2, 2, 7),
    }));
    assert_ne!(id, id2);
}
