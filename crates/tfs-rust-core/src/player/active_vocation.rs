//! Stored vs active vocation — 772 premium promotion gating.
//!
//! When `premiumPromotion = true` (default, 772 corpus), free accounts use the base
//! vocation row for mechanics/look while stored promotion stays in the save file.
//! When `premiumPromotion = false`, promotion is not premium-gated — free accounts
//! keep full promoted vocation and perks.

use crate::config::{ConfigManager, get_bool_or};
use crate::creature::CreatureKind;
use crate::creature::vocation::VocationProfile;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use tfs_rust_content::vocations::VocationRegistry;

/// `config.lua` `premiumPromotion` — default true (772 corpus).
pub fn premium_promotion_enabled(config: &ConfigManager) -> bool {
    get_bool_or(config, "premiumPromotion", true).unwrap_or(true)
}

/// Stored row is a promoted vocation (TFS ids 5–8; `from_vocation != id`).
pub fn is_stored_promoted(vocations: &VocationRegistry, stored_id: i32) -> bool {
    let Some(def) = vocations.get(stored_id) else {
        return false;
    };
    def.from_vocation != 0 && def.from_vocation != def.id
}

/// `GetEffectiveProfession` — demote stored promotion for mechanics/display.
pub fn effective_vocation_id(vocations: &VocationRegistry, stored_id: i32) -> i32 {
    if is_stored_promoted(vocations, stored_id) {
        vocations.demotion_id(stored_id).unwrap_or(stored_id)
    } else {
        stored_id
    }
}

/// `GetActiveProfession` — premium keeps stored id; free uses effective when enabled.
pub fn active_vocation_id(
    vocations: &VocationRegistry,
    stored_id: i32,
    premium: bool,
    premium_promotion: bool,
) -> i32 {
    if premium_promotion && !premium {
        effective_vocation_id(vocations, stored_id)
    } else {
        stored_id
    }
}

/// `GetActivePromotion` — stored promotion row with perks active.
pub fn active_promotion(
    vocations: &VocationRegistry,
    stored_id: i32,
    premium: bool,
    premium_promotion: bool,
) -> bool {
    if !is_stored_promoted(vocations, stored_id) {
        return false;
    }
    if premium_promotion { premium } else { true }
}

fn profile_for_active_vocation(
    vocations: &VocationRegistry,
    stored_id: i32,
    premium: bool,
    premium_promotion: bool,
) -> VocationProfile {
    let active_id = active_vocation_id(vocations, stored_id, premium, premium_promotion);
    vocations
        .get(active_id)
        .map(VocationProfile::from_def)
        .unwrap_or_else(VocationProfile::none_vocation)
}

impl GameWorld {
    pub(crate) fn premium_promotion_enabled(&self) -> bool {
        premium_promotion_enabled(&self.config)
    }

    pub(crate) fn player_stored_vocation_id(&self, cid: CreatureId) -> Option<i32> {
        match self.creatures.get(cid)? {
            CreatureKind::Player(p) => Some(p.vocation_id),
            _ => None,
        }
    }

    /// Active vocation for mechanics, look, and item gates (`GetActiveProfession`).
    pub(crate) fn player_active_vocation_id(&self, cid: CreatureId) -> Option<i32> {
        let stored = self.player_stored_vocation_id(cid)?;
        let premium = self.player_is_premium(cid);
        Some(active_vocation_id(
            &self.vocations,
            stored,
            premium,
            self.premium_promotion_enabled(),
        ))
    }

    /// Promotion perks (soul cap/tick, death loss %) when corpus mode is on.
    pub(crate) fn player_active_promotion(&self, cid: CreatureId) -> bool {
        let Some(stored) = self.player_stored_vocation_id(cid) else {
            return false;
        };
        active_promotion(
            &self.vocations,
            stored,
            self.player_is_premium(cid),
            self.premium_promotion_enabled(),
        )
    }

    /// Sync `vocation_profile` to the active row; stored `vocation_id` unchanged.
    pub(crate) fn refresh_player_active_vocation_profile(&mut self, cid: CreatureId) {
        let Some(stored) = self.player_stored_vocation_id(cid) else {
            return;
        };
        let premium = self.player_is_premium(cid);
        let premium_promotion = self.premium_promotion_enabled();
        let had_soul_timer = self
            .creatures
            .get(cid)
            .is_some_and(|k| matches!(k, CreatureKind::Player(p) if p.soul_max_count > 0));
        let profile =
            profile_for_active_vocation(&self.vocations, stored, premium, premium_promotion);
        let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) else {
            return;
        };
        p.vocation_profile = profile;
        let soul_max = p.vocation_profile.soul_max.max(0);
        p.economy.soul = p.economy.soul.min(soul_max);
        if had_soul_timer {
            p.arm_soul_regen_timer();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigManager;
    use std::fs;
    use std::path::PathBuf;
    use tfs_rust_content::vocations::VocationRegistry;

    fn test_vocations() -> VocationRegistry {
        VocationRegistry::load(std::path::Path::new("data/defs/vocations.lua"))
            .expect("vocations.lua")
    }

    fn temp_config(contents: &str) -> ConfigManager {
        let mut path = PathBuf::from(std::env::temp_dir());
        path.push(format!("tfs-premium-promotion-{}.lua", std::process::id()));
        fs::write(&path, contents).expect("write temp config");
        ConfigManager::load(&path).expect("load temp config")
    }

    #[test]
    fn free_account_uses_effective_vocation_when_enabled() {
        let vocations = test_vocations();
        assert_eq!(
            active_vocation_id(&vocations, 8, false, true),
            4,
            "elite knight → knight when free"
        );
        assert!(!active_promotion(&vocations, 8, false, true));
    }

    #[test]
    fn premium_account_keeps_stored_vocation_when_enabled() {
        let vocations = test_vocations();
        assert_eq!(active_vocation_id(&vocations, 8, true, true), 8);
        assert!(active_promotion(&vocations, 8, true, true));
    }

    #[test]
    fn toggle_off_lets_free_accounts_stay_promoted() {
        let vocations = test_vocations();
        assert_eq!(
            active_vocation_id(&vocations, 8, false, false),
            8,
            "free keeps elite knight when promotion is not premium-gated"
        );
        assert!(active_promotion(&vocations, 8, false, false));
    }

    #[test]
    fn config_defaults_premium_promotion_true() {
        let cfg = temp_config("freePremium = false");
        assert!(premium_promotion_enabled(&cfg));
    }

    #[test]
    fn config_reads_premium_promotion_false() {
        let cfg = temp_config("premiumPromotion = false");
        assert!(!premium_promotion_enabled(&cfg));
    }
}
