//! Idle NPC voices.
//!
//! Pack surface: TFS `NpcType:addVoice` / `VoiceModule` interval and chance.
//! Spoken only while the NPC is idle and not in a conversation.

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;

/// Whether a voice line may be spoken on this tick.
pub fn voice_due(engaged: bool, now_ms: u64, next_ms: u64, roll: u32, chance: u32) -> bool {
    !engaged && chance > 0 && now_ms >= next_ms && roll < chance
}

impl GameWorld {
    /// Consider one idle voice line. A failed chance still advances that line's interval.
    pub(crate) fn npc_voice_tick(&mut self, npc_id: CreatureId) {
        let (def_id, engaged) = match self.creatures.get(npc_id) {
            Some(CreatureKind::Npc(n)) => (n.definition, n.runtime.is_engaged()),
            _ => return,
        };
        if engaged {
            return;
        }
        let voices = self
            .npcs_db
            .get(def_id)
            .map(|def| def.voices.clone())
            .unwrap_or_default();
        if voices.is_empty() {
            return;
        }
        let now = self.server_ms;
        if let Some(CreatureKind::Npc(n)) = self.creatures.get_mut(npc_id)
            && n.runtime.next_voice_at.len() < voices.len()
        {
            n.runtime.next_voice_at.resize(voices.len(), 0);
        }
        let nexts: Vec<u64> = match self.creatures.get(npc_id) {
            Some(CreatureKind::Npc(n)) => (0..voices.len())
                .map(|i| n.runtime.next_voice_at.get(i).copied().unwrap_or(0))
                .collect(),
            _ => return,
        };
        let mut spoken: Option<String> = None;
        let mut advance = Vec::new();
        for (i, voice) in voices.iter().enumerate() {
            let next = nexts[i];
            let roll = self.parity_rand_mod(100);
            let wait = u64::from(voice.interval_ms.max(1));
            if voice_due(false, now, next, roll, voice.chance) {
                spoken = Some(voice.text.clone());
                advance.push((i, now.saturating_add(wait)));
                break;
            }
            if now >= next {
                advance.push((i, now.saturating_add(wait)));
            }
        }
        if let Some(CreatureKind::Npc(n)) = self.creatures.get_mut(npc_id) {
            for (index, at) in advance {
                if let Some(slot) = n.runtime.next_voice_at.get_mut(index) {
                    *slot = at;
                }
            }
        }
        if let Some(text) = spoken {
            self.enqueue_creature_talk(npc_id, text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::voice_due;

    #[test]
    fn voice_line_does_not_fire_while_talking() {
        assert!(!voice_due(true, 5_000, 0, 0, 100));
        assert!(voice_due(false, 5_000, 0, 0, 100));
        assert!(!voice_due(false, 100, 5_000, 0, 100));
        assert!(!voice_due(false, 5_000, 0, 40, 10));
    }
}
