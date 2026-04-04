//! Item payload as in `NetworkMessage::addItem(uint16_t id, uint8_t count, ...)`.
// C++ reference (this repo): `src/networkmessage.cpp`.

use crate::NetworkMessage;

/// Template item (no live `Item*`): matches `addItem(id, count, false)` non-splash, non-animation path.
pub fn write_item_template(msg: &mut NetworkMessage, client_id: u16, count: u8, stackable: bool) {
    msg.write_u16(client_id);
    msg.write_u8(0xFF); // MARK_UNMARKED
    if stackable {
        msg.write_u8(count);
    }
    msg.write_u8(0x00); // duration — template / no duration
}
