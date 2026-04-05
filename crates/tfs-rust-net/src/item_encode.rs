//! Item payload as in `NetworkMessage::addItem(uint16_t id, uint8_t count, ...)`.
// C++ reference (this repo): `src/networkmessage.cpp`.
//
// **Wire IDs:** every `client_id` / `item_client_id` parameter here is the **client (sprite) id**
// from `Item::items[id].clientId`, never the server item id (`addItem` writes `it.clientId`).

use crate::NetworkMessage;

/// `fluidMap` in `src/const.h` — used when `it.isSplash() || it.isFluidContainer()`.
const FLUID_MAP: [u8; 8] = [0, 1, 5, 3, 6, 8, 9, 2];

#[inline]
fn fluid_map_byte(count: u8) -> u8 {
    FLUID_MAP[(count & 7) as usize]
}

/// Inverse of `fluid_map_byte` — `serverFluidToClient` (`src/tools.cpp`).
#[inline]
pub fn server_fluid_to_client(server_fluid: u8) -> u8 {
    for (i, &v) in FLUID_MAP.iter().enumerate() {
        if v == server_fluid {
            return i as u8;
        }
    }
    0
}

/// Template item (no live `Item*`): matches `NetworkMessage::addItem(uint16_t id, uint8_t count, …)`
/// except for the **trailing duration byte** (see below).
///
/// Writes stack count or fluid sub-type, optional `0xFE` animation phase, then when
/// `with_description` (OTCv8): empty `addString("")` (`src/networkmessage.cpp` ~109–110).
///
/// **Deviation from C++ (`networkmessage.cpp` L113–114):** TFS appends `addByte(0x00)` (“duration”
/// for templates). **OTClient v8** `ProtocolGame::getItem` does **not** consume that byte when
/// `GameDisplayItemDuration` (129) is off — which is the default — so the byte shifts every later
/// field (`docs/OTCLIENT_INFO.md`). We omit it for wire compatibility with stock OTClient v8.
pub fn write_item_template(
    msg: &mut NetworkMessage,
    client_id: u16,
    count: u8,
    stackable: bool,
    is_splash_or_fluid: bool,
    is_animation: bool,
    with_description: bool,
) {
    msg.write_u16(client_id);
    msg.write_u8(0xFF); // MARK_UNMARKED
    if stackable {
        msg.write_u8(count);
    } else if is_splash_or_fluid {
        msg.write_u8(fluid_map_byte(count));
    }
    if is_animation {
        msg.write_u8(0xFE); // random phase (C++ `it.isAnimation`)
    }
    if with_description {
        msg.write_string("");
    }
}

/// Byte length of [`write_item_template`] for the same arguments (keep in sync when encoding changes).
#[inline]
pub fn item_template_wire_len(
    _client_id: u16,
    _count: u8,
    stackable: bool,
    is_splash_or_fluid: bool,
    is_animation: bool,
    with_description: bool,
) -> usize {
    let mut n = 2 + 1; // client id + MARK_UNMARKED
    if stackable {
        n += 1;
    } else if is_splash_or_fluid {
        n += 1;
    }
    if is_animation {
        n += 1;
    }
    if with_description {
        n += 2; // empty string: u16 length 0
    }
    n
}

/// Live `Item*` serialization (`NetworkMessage::addItem(const Item* item, bool withDescription)`).
/// `duration_pickup`: `Some((duration, stop_time_byte))` for non-stackable timed pickup items (C++ branch).
// C++ reference: `src/networkmessage.cpp` L117–152.
pub fn write_item_live(
    msg: &mut NetworkMessage,
    client_id: u16,
    count: u8,
    stackable: bool,
    is_splash_or_fluid: bool,
    is_animation: bool,
    with_description: bool,
    description: &str,
    duration_pickup: Option<(u32, u8)>,
) {
    msg.write_u16(client_id);
    msg.write_u8(0xFF); // MARK_UNMARKED
    if stackable {
        msg.write_u8(count.min(0xFF));
    } else if is_splash_or_fluid {
        msg.write_u8(fluid_map_byte(count));
    }
    if is_animation {
        msg.write_u8(0xFE);
    }
    if with_description {
        msg.write_string(description);
    }
    if stackable {
        msg.write_u8(0x00);
    } else if let Some((dur, stop)) = duration_pickup {
        msg.write_u8(0x01);
        msg.write_u32(dur);
        msg.write_u8(stop);
    } else {
        msg.write_u8(0x00);
    }
}

#[cfg(test)]
mod tests {
    use super::{item_template_wire_len, write_item_live, write_item_template};
    use crate::NetworkMessage;

    #[test]
    fn template_wire_len_matches_write() {
        for &(cid, stack, splash, anim, desc) in &[
            (0x1234u16, false, false, false, false),
            (0x1234u16, false, false, false, true),
            (0x1234u16, false, false, true, false),
            (0x1234u16, true, false, false, false),
            (0x1234u16, false, true, false, false),
        ] {
            let mut m = NetworkMessage::new();
            write_item_template(&mut m, cid, 3, stack, splash, anim, desc);
            assert_eq!(
                m.as_bytes().len(),
                item_template_wire_len(cid, 3, stack, splash, anim, desc),
                "wire_len must stay in sync with write_item_template"
            );
        }
    }

    #[test]
    fn template_non_animated_matches_add_item_minimal() {
        let mut m = NetworkMessage::new();
        write_item_template(&mut m, 0x1234, 1, false, false, false, false);
        assert_eq!(m.as_bytes(), &[0x34, 0x12, 0xFF]);
    }

    #[test]
    fn template_with_description_otcv8_inserts_empty_string() {
        let mut m = NetworkMessage::new();
        write_item_template(&mut m, 0x1234, 1, false, false, false, true);
        assert_eq!(m.as_bytes(), &[0x34, 0x12, 0xFF, 0x00, 0x00]);
    }

    #[test]
    fn template_animated_inserts_random_phase_byte() {
        let mut m = NetworkMessage::new();
        write_item_template(&mut m, 0x1234, 1, false, false, true, false);
        assert_eq!(m.as_bytes(), &[0x34, 0x12, 0xFF, 0xFE]);
    }

    #[test]
    fn template_animated_with_description_inserts_string() {
        let mut m = NetworkMessage::new();
        write_item_template(&mut m, 0x1234, 1, false, false, true, true);
        assert_eq!(m.as_bytes(), &[0x34, 0x12, 0xFF, 0xFE, 0x00, 0x00]);
    }

    #[test]
    fn template_fluid_writes_subtype() {
        let mut m = NetworkMessage::new();
        // count 3 → fluidMap[3] = 3 (CLIENTFLUID_BROWN_1) per `src/const.h`
        write_item_template(&mut m, 0x1234, 3, false, true, false, false);
        assert_eq!(m.as_bytes(), &[0x34, 0x12, 0xFF, 0x03]);
    }

    #[test]
    fn live_stackable_with_description_ends_with_duration_zero() {
        let mut m = NetworkMessage::new();
        write_item_live(
            &mut m,
            0x1234,
            7,
            true,
            false,
            false,
            true,
            "desc",
            None,
        );
        // client + 0xFF + count + string(desc) + 0x00 duration
        assert!(m.as_bytes().ends_with(&[0x00]));
    }

    #[test]
    fn live_non_stackable_duration_pickup_matches_cpp() {
        let mut m = NetworkMessage::new();
        write_item_live(
            &mut m,
            0x1234,
            1,
            false,
            false,
            false,
            false,
            "",
            Some((12345, 1)),
        );
        assert_eq!(
            m.as_bytes(),
            &[0x34, 0x12, 0xFF, 0x01, 0x39, 0x30, 0x00, 0x00, 0x01]
        );
    }
}
