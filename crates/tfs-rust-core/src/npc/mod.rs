//! NPC dialogue runtime — matching, focus/queue, speech stimulus, mutations (NPC-4/5).
//!
//! Domain: TFS-style `Npc` / `NpcDialogue` execution.
//! 772 outcomes: `TNPC::TalkStimulus` / `IdleStimulus` / `TBehaviourDatabase::react`
//! (`crnonpl.cc`), `SearchForWord` / `SearchForNumber` (`strings.cc`),
//! speech fan-out `TFindCreatures(3,3,…,FIND_NPCS)` (`operate.cc`).

mod actions;
mod events;
mod expr;
mod focus;
mod host;
mod match_rule;
mod react;
mod stimulus;
mod words;

#[cfg(test)]
mod tests;

pub use actions::NpcActionHost;
pub use events::{DialogueEvent, DialogueSituationKind, DialogueTrace, MutateOp};
pub(crate) use focus::deliver_npc_say_stimuli;
#[allow(unused_imports)]
pub use match_rule::{match_dialogue_rule, MatchCaptures, RuleMatch};
#[allow(unused_imports)]
pub use react::{apply_dialogue_plan, DialoguePlan, PlannedReply, ReactMeta};
#[allow(unused_imports)]
pub use stimulus::collect_npc_speech_candidates;
#[allow(unused_imports)]
pub use words::{search_for_number, search_for_word};
