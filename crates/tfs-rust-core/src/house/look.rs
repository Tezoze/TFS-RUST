//! House-door look text.
//!
//! Corpus: 772 `operate.cc` NAMEDOOR (~2091–2111) concatenates
//! `. It belongs to house '%s'. %s owns this house` before the name-sentence period.
//! Pack: TFS/TVP `House::updateDoorDescription` uses the same sentence as a special
//! description (newline + optional gold). Native look follows the corpus line; no
//! `HOUSE_DOOR_SHOW_PRICE`.

use crate::game_world::GameWorld;
use crate::tile::Tile;
use tfs_rust_common::Position;

/// Corpus NAMEDOOR clause, including the leading `. ` that follows the item name.
fn namedoor_house_clause(house_name: &str, owner_name: &str) -> String {
    let owner = if owner_name.is_empty() {
        "Nobody"
    } else {
        owner_name
    };
    format!(". It belongs to house '{house_name}'. {owner} owns this house")
}

/// Insert the NAMEDOOR clause before the first `.` of `Item::getDescription` output.
///
/// `a closed door.` → `a closed door. It belongs to house 'X'. Y owns this house.`
pub fn insert_namedoor_house_clause(description: &str, house_name: &str, owner_name: &str) -> String {
    let clause = namedoor_house_clause(house_name, owner_name);
    match description.find('.') {
        Some(idx) => {
            let (before, after) = description.split_at(idx);
            format!("{before}{clause}{after}")
        }
        None => format!("{description}{clause}."),
    }
}

impl GameWorld {
    /// Door on a `Tile::House`: corpus `GetHouseID` + `NAMEDOOR` (`operate.cc` ~2091).
    /// Inventory/container looks (`pos.x == 0xFFFF`) must not call this.
    pub fn apply_namedoor_house_look(&self, pos: Position, description: String) -> String {
        let Some((house_name, owner_name)) = self.house_namedoor_look_names(pos) else {
            return description;
        };
        insert_namedoor_house_clause(&description, &house_name, &owner_name)
    }

    fn house_namedoor_look_names(&self, pos: Position) -> Option<(String, String)> {
        let Tile::House(h) = self.map.get_tile(pos)? else {
            return None;
        };
        let rec = self.houses.records.get(&h.house_id)?;
        if rec.name.is_empty() {
            return None;
        }
        Some((rec.name.clone(), self.house_owner_look_name(h.house_id)))
    }

    /// `GetHouseOwner` / TFS `House::ownerName` — `"Nobody"` when unowned or unresolved.
    fn house_owner_look_name(&self, house_id: u32) -> String {
        let Some(access) = self.houses.houses.get(&house_id) else {
            return "Nobody".to_string();
        };
        let Some(guid) = access.owner_guid else {
            return "Nobody".to_string();
        };
        if !access.owner_name.is_empty() {
            return access.owner_name.clone();
        }
        if let Some(&cid) = self.player_by_guid.get(&guid)
            && let Some(k) = self.creatures.get(cid)
        {
            return k.base().name.clone();
        }
        "Nobody".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namedoor_clause_unowned() {
        assert_eq!(
            insert_namedoor_house_clause("a closed door.", "Spiritkeep", ""),
            "a closed door. It belongs to house 'Spiritkeep'. Nobody owns this house."
        );
    }

    #[test]
    fn namedoor_clause_keeps_locked_info_line() {
        assert_eq!(
            insert_namedoor_house_clause("a closed door.\nIt is locked.", "Spiritkeep", "Alice"),
            "a closed door. It belongs to house 'Spiritkeep'. Alice owns this house.\nIt is locked."
        );
    }

    #[test]
    fn namedoor_clause_without_period() {
        assert_eq!(
            insert_namedoor_house_clause("a closed door", "Sunset Homes, Flat 01", "Bob"),
            "a closed door. It belongs to house 'Sunset Homes, Flat 01'. Bob owns this house."
        );
    }
}
