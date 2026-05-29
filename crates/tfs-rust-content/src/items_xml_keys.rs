//! Known `items.xml` `<attribute key="...">` names for warning suppression.
//!
//! C++ reference: `src/items.cpp` `ItemParseAttributesMap` (lines 16–139), plus Rust-only
//! `extradefense` (alias of `extradef`) and `blockprojectile`.
//!
//! **Phase 3 (abilities):** all keys in the `ItemParseAttributesMap` abilities block (equipment `speed`, skills, absorb, suppress, elements, …) are parsed into [crate::item_abilities::ItemAbilities] on [crate::otb::ItemType::abilities] — see [crate::item_abilities::apply_ability_attribute].
//! **Phase 2 (still `xml_attributes` + typed `ItemType` where noted):** `fluidsource`, `decayto` / `duration` / `stopduration`, `transformto` / `destroyto` / equip transforms, `maletransformto` / `femaletransformto`, `shoottype`, `effect`, `corpsetype`, `field` (+ nested), `supply`, `showcharges`, `showduration`, `showattributes`, `leveldoor`, `partnerdirection`, `writeonceitemid`, `runespellname`, … — keep this list in sync. Details: `tasks/items-parsing-audit.md` (Phase 2–3).

/// Sorted for binary search — keep alphabetical when adding keys.
const KNOWN_XML_KEYS: &[&str] = &[
    "absorbpercentall",
    "absorbpercentallelements",
    "absorbpercentdeath",
    "absorbpercentdrown",
    "absorbpercentearth",
    "absorbpercentelements",
    "absorbpercentenergy",
    "absorbpercentfire",
    "absorbpercenthealing",
    "absorbpercentholy",
    "absorbpercentice",
    "absorbpercentlifedrain",
    "absorbpercentmagic",
    "absorbpercentmanadrain",
    "absorbpercentphysical",
    "absorbpercentpoison",
    "absorbpercentundefined",
    "allowdistread",
    "allowpickupable",
    "ammotype",
    "armor",
    "attack",
    "attackspeed",
    "blocking",
    "blockprojectile",
    "charges",
    "containersize",
    "corpsetype",
    "criticalhitamount",
    "criticalhitchance",
    "decayto",
    "defense",
    "description",
    "destroyto",
    "duration",
    "effect",
    "elementdeath",
    "elementearth",
    "elementenergy",
    "elementfire",
    "elementholy",
    "elementice",
    "extradef",
    "extradefense",
    "femalesleeper",
    "femaletransformto",
    "field",
    "fieldabsorbpercentearth",
    "fieldabsorbpercentenergy",
    "fieldabsorbpercentfire",
    "fieldabsorbpercentpoison",
    "floorchange",
    "fluidsource",
    "forcesave",
    "forceserialize",
    "healthgain",
    "healthticks",
    "hitchance",
    "invisible",
    "leveldoor",
    "levelrequired",
    "lifeleechamount",
    "lifeleechchance",
    "magiclevelpoints",
    "magiclevelrequired",
    "magicpoints",
    "magicpointspercent",
    "malesleeper",
    "maletransformto",
    "managain",
    "manaleechamount",
    "manaleechchance",
    "manashield",
    "manaticks",
    "maxhitchance",
    "maxhitpoints",
    "maxhitpointspercent",
    "maxmanapoints",
    "maxmanapointspercent",
    "maxtextlen",
    "movable",
    "moveable",
    "partnerdirection",
    "pickupable",
    "range",
    "readable",
    "replaceable",
    "rotateto",
    "runespellname",
    "shoottype",
    "showattributes",
    "showcharges",
    "showcount",
    "showduration",
    "skillaxe",
    "skillclub",
    "skilldist",
    "skillfish",
    "skillfist",
    "skillshield",
    "skillsword",
    "slottype",
    "speed",
    "stopduration",
    "storeitem",
    "supply",
    "suppresscurse",
    "suppressdazzle",
    "suppressdrown",
    "suppressdrunk",
    "suppressenergy",
    "suppressfire",
    "suppressfreeze",
    "suppressphysical",
    "suppresspoison",
    "transformdeequipto",
    "transformequipto",
    "transformto",
    "type",
    "vocation",
    "walkstack",
    "weapontype",
    "weight",
    "writeable",
    "writeonceitemid",
];

#[inline]
pub(crate) fn is_known_xml_key(k: &str) -> bool {
    KNOWN_XML_KEYS.binary_search(&k).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_keys_sorted() {
        let mut sorted = KNOWN_XML_KEYS.to_vec();
        sorted.sort();
        assert_eq!(
            sorted.as_slice(),
            KNOWN_XML_KEYS,
            "KNOWN_XML_KEYS must stay sorted for binary_search"
        );
    }

    #[test]
    fn samples_recognized() {
        assert!(is_known_xml_key("blocking"));
        assert!(is_known_xml_key("hitchance"));
        assert!(!is_known_xml_key("not_a_real_tfs_key_xyz"));
    }
}
