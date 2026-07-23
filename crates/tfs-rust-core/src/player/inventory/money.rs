//! TFS-shaped money helpers with 772 denomination / change outcomes.
//!
//! Coin ids: gold `2148`×1, platinum `2152`×100, crystal `2160`×10000.
//! C++: `TNPC::GiveMoney` / `GetMoney` (`crnonpl.cc:1904-1934`),
//! `CalculateChange` (`info.cc:634-687`).

use slotmap::Key;

use crate::game_world::GameWorld;
use crate::ids::CreatureId;

/// 772 / TFS gold coin.
pub const ITEM_GOLD_COIN: u16 = 2148;
/// 772 / TFS platinum coin.
pub const ITEM_PLATINUM_COIN: u16 = 2152;
/// 772 / TFS crystal coin.
pub const ITEM_CRYSTAL_COIN: u16 = 2160;

const GOLD_WORTH: u64 = 1;
const PLATINUM_WORTH: u64 = 100;
const CRYSTAL_WORTH: u64 = 10_000;

impl GameWorld {
    /// Total gold-equivalent of gold/platinum/crystal coins in inventory.
    pub fn player_count_money(&self, cid: CreatureId) -> u64 {
        let gold = u64::from(self.player_get_item_type_count(cid, ITEM_GOLD_COIN, -1));
        let plat = u64::from(self.player_get_item_type_count(cid, ITEM_PLATINUM_COIN, -1));
        let crystal = u64::from(self.player_get_item_type_count(cid, ITEM_CRYSTAL_COIN, -1));
        gold * GOLD_WORTH + plat * PLATINUM_WORTH + crystal * CRYSTAL_WORTH
    }

    /// 772 `TNPC::GiveMoney` — split amount into crystal/platinum/gold and add stacks.
    pub fn player_create_money(&mut self, cid: CreatureId, amount: i32) -> Result<(), String> {
        if amount <= 0 {
            return Ok(());
        }
        let amount = amount as u32;
        let crystal = amount / 10_000;
        let rem = amount % 10_000;
        let platinum = rem / 100;
        let gold = rem % 100;
        if crystal > 0 {
            self.player_add_item_count(cid, ITEM_CRYSTAL_COIN, crystal)?;
        }
        if platinum > 0 {
            self.player_add_item_count(cid, ITEM_PLATINUM_COIN, platinum)?;
        }
        if gold > 0 {
            self.player_add_item_count(cid, ITEM_GOLD_COIN, gold)?;
        }
        Ok(())
    }

    /// 772 `TNPC::GetMoney` — remove coins via [`calculate_change`], giving change when needed.
    pub fn player_delete_money(&mut self, cid: CreatureId, amount: i32) -> Result<(), String> {
        if amount <= 0 {
            return Ok(());
        }
        let amount = amount as u32;
        if self.player_count_money(cid) < u64::from(amount) {
            return Err(format!("insufficient money for {amount}"));
        }
        let mut gold = self.player_get_item_type_count(cid, ITEM_GOLD_COIN, -1) as i32;
        let mut platinum = self.player_get_item_type_count(cid, ITEM_PLATINUM_COIN, -1) as i32;
        let mut crystal = self.player_get_item_type_count(cid, ITEM_CRYSTAL_COIN, -1) as i32;
        calculate_change(amount as i32, &mut gold, &mut platinum, &mut crystal);

        apply_coin_delta(self, cid, ITEM_GOLD_COIN, gold)?;
        apply_coin_delta(self, cid, ITEM_PLATINUM_COIN, platinum)?;
        apply_coin_delta(self, cid, ITEM_CRYSTAL_COIN, crystal)?;
        Ok(())
    }

    /// Add `count` of `item_id` to the player (backpack / wherever / map drop).
    pub fn player_add_item_count(
        &mut self,
        cid: CreatureId,
        item_id: u16,
        count: u32,
    ) -> Result<(), String> {
        if count == 0 {
            return Ok(());
        }
        let ffi = cid.data().as_ffi();
        // Stackable coins come in stacks of up to 100; loop for large amounts.
        let mut remaining = count;
        while remaining > 0 {
            let chunk = remaining.min(100);
            self.lua_script_player_add_item_full(ffi, item_id, chunk, -1, true, 0)
                .map_err(|e| e)?
                .ok_or_else(|| format!("failed to add item {item_id} x{chunk}"))?;
            remaining -= chunk;
        }
        Ok(())
    }
}

fn apply_coin_delta(
    world: &mut GameWorld,
    cid: CreatureId,
    item_id: u16,
    delta: i32,
) -> Result<(), String> {
    if delta > 0 {
        if !world.player_remove_item_of_type(cid, item_id, delta as u32, -1, false) {
            return Err(format!("failed to remove {delta} of item {item_id}"));
        }
    } else if delta < 0 {
        world.player_add_item_count(cid, item_id, (-delta) as u32)?;
    }
    Ok(())
}

/// 772 `CalculateChange` — mutates coin counts to the remove(+)/give-change(−) deltas.
///
/// C++ `info.cc:634-687`. On insufficient funds the C++ function logs and returns
/// without modifying; callers must pre-check total worth.
pub fn calculate_change(amount: i32, gold: &mut i32, platinum: &mut i32, crystal: &mut i32) {
    let go = *gold;
    let pl = *platinum;
    let cr = *crystal;

    if (cr as i64 * 10_000 + pl as i64 * 100 + go as i64) < i64::from(amount) {
        return;
    }

    let amount_cr = amount / 10_000;
    let mut amount_rem = amount % 10_000;
    let (out_cr, out_pl, out_go) = if (pl * 100 + go) < amount_rem {
        let c = amount_cr + 1;
        let rem = amount_rem - 10_000;
        (c, rem / 100, rem % 100)
    } else {
        let c = if cr < amount_cr {
            amount_rem = amount - cr * 10_000;
            cr
        } else {
            amount_cr
        };
        let amount_pl = amount_rem / 100;
        let amount_go = amount_rem % 100;
        if go < amount_go {
            (c, amount_pl + 1, amount_go - 100)
        } else if pl < amount_pl {
            (c, pl, amount_rem - pl * 100)
        } else {
            (c, amount_pl, amount_go)
        }
    };

    *gold = out_go;
    *platinum = out_pl;
    *crystal = out_cr;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculate_change_exact_gold() {
        let mut g = 50;
        let mut p = 0;
        let mut c = 0;
        calculate_change(30, &mut g, &mut p, &mut c);
        assert_eq!((g, p, c), (30, 0, 0));
    }

    #[test]
    fn calculate_change_breaks_platinum() {
        // 1 platinum, pay 30 → remove 1 plat, give 70 gold change → deltas +1 plat, −70 gold
        let mut g = 0;
        let mut p = 1;
        let mut c = 0;
        calculate_change(30, &mut g, &mut p, &mut c);
        assert_eq!(p, 1);
        assert_eq!(g, -70);
        assert_eq!(c, 0);
    }

    #[test]
    fn calculate_change_insufficient_leaves_unchanged() {
        let mut g = 5;
        let mut p = 0;
        let mut c = 0;
        calculate_change(100, &mut g, &mut p, &mut c);
        assert_eq!((g, p, c), (5, 0, 0));
    }
}
