//! Offline mailbox delivery — serialize now, append DB off the game thread, splice on login.
//!
//! C++ reference: `moveuse.cc:712-919` `SendMail` / `SendMails`.
//! Does **not** use `houses.pending_depot_dumps` (that map is house eviction / welcome letters).
//!
//! One outbox *queue* per guid (multiple letters accumulate). At most one DB
//! append is in flight per guid so overlapping load-append-save cannot
//! last-writer-wins. Failed appends stay queued for retry / login splice;
//! successful rows stay until login consumes them so a stale `PlayerLoaded`
//! can still splice.

use tfs_rust_common::GameCommand;
use tfs_rust_db::items::{ItemRecord, ItemStore, ItemTable};
use tfs_rust_db::player::LoadedPlayerData;

use crate::game_world::GameWorld;
use crate::game_world_save::append_save_item_tree;
use crate::ids::ItemId;

const MAX_APPEND_FAILS: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MailPersistState {
    Queued,
    InFlight,
    Acked,
}

/// One serialized letter/parcel waiting for DB ack and/or login splice.
#[derive(Debug, Clone)]
pub struct PendingMail {
    pub town_id: u32,
    pub records: Vec<ItemRecord>,
    pub item_ids: Vec<ItemId>,
    pub delivered_live: bool,
    persist: MailPersistState,
}

/// Per-guid offline-mail queue.
#[derive(Debug, Default)]
pub struct MailOutbox {
    pub pending: Vec<PendingMail>,
    /// Sid/pid/itemtype actually written by the last successful append.
    last_appended: Vec<(i32, i32, u16)>,
    fail_count: u8,
}

impl MailOutbox {
    fn has_inflight(&self) -> bool {
        self.pending
            .iter()
            .any(|p| p.persist == MailPersistState::InFlight)
    }
}

/// `PlayerLoaded` held until [`GameCommand::MailDeliveryFinished`].
pub struct DeferredLogin {
    pub conn_id: tfs_rust_common::ConnId,
    pub name: String,
    pub operating_system: u16,
    pub otclient_v8: u16,
    pub peer_ip: u32,
    pub loaded: LoadedPlayerData,
}

impl GameWorld {
    /// Detach+stamp already done by caller. Serialize the tree now and spawn DB append.
    pub(crate) fn queue_offline_mail(&mut self, item_id: ItemId, guid: u32, town_id: u32) -> bool {
        let roots = vec![(town_id as i32, item_id)];
        let mut records: Vec<ItemRecord> = Vec::new();
        if append_save_item_tree(self, &roots, &mut records).is_err() {
            return false;
        }
        self.mail_outbox.entry(guid).or_default().pending.push(PendingMail {
            town_id,
            records,
            item_ids: vec![item_id],
            delivered_live: false,
            persist: MailPersistState::Queued,
        });
        self.spawn_mail_append(guid);
        true
    }

    /// Splice pending records into a loaded character's depot (login-before-ack).
    pub(crate) fn splice_mail_into_loaded(loaded: &mut LoadedPlayerData, records: &[ItemRecord]) {
        crate::depot_append::append_offset_records(&mut loaded.items.depot, records.to_vec());
    }

    pub(crate) fn splice_outbox_into_loaded(&self, guid: u32, loaded: &mut LoadedPlayerData) {
        let Some(outbox) = self.mail_outbox.get(&guid) else {
            return;
        };
        for pending in &outbox.pending {
            Self::splice_mail_into_loaded(loaded, &pending.records);
        }
    }

    /// Defer only while a DB append is in flight (stale `PlayerLoaded` would miss the write).
    pub(crate) fn mail_login_should_defer(&self, guid: u32) -> bool {
        if self.player_by_guid.contains_key(&guid) {
            return false;
        }
        self.mail_outbox
            .get(&guid)
            .is_some_and(|b| b.has_inflight())
    }

    /// Fresh login after ack: splice if the load missed the append, else drop (no duplicate).
    pub(crate) fn apply_outbox_to_loaded(&mut self, guid: u32, loaded: &mut LoadedPlayerData) {
        let Some(outbox) = self.mail_outbox.get(&guid) else {
            return;
        };
        if outbox.pending.iter().all(|p| p.delivered_live) {
            self.consume_mail_outbox(guid);
            return;
        }
        let already_in_depot = !outbox.last_appended.is_empty()
            && outbox.last_appended.iter().all(|(pid, sid, ty)| {
                loaded
                    .items
                    .depot
                    .iter()
                    .any(|r| r.pid == *pid && r.sid == *sid && r.itemtype == *ty)
            });
        if !already_in_depot {
            self.splice_outbox_into_loaded(guid, loaded);
        }
        self.consume_mail_outbox(guid);
    }

    pub(crate) fn consume_mail_outbox(&mut self, guid: u32) {
        if let Some(outbox) = self.mail_outbox.remove(&guid) {
            for pending in outbox.pending {
                if !pending.delivered_live {
                    self.items_pending_release.extend(pending.item_ids);
                }
            }
        }
    }

    /// TakeOver: live town depot wins; mark so ack does not splice a duplicate.
    pub(crate) fn deliver_pending_mail_live(&mut self, cid: crate::ids::CreatureId, guid: u32) {
        let Some(outbox) = self.mail_outbox.get_mut(&guid) else {
            return;
        };
        let jobs: Vec<(u32, Vec<ItemId>)> = outbox
            .pending
            .iter_mut()
            .filter(|p| !p.delivered_live)
            .map(|p| {
                p.delivered_live = true;
                (p.town_id, p.item_ids.clone())
            })
            .collect();
        let inflight = outbox.has_inflight();
        for (town_id, ids) in jobs {
            for item_id in ids {
                self.house_add_item_to_town_depot(cid, town_id, item_id);
            }
        }
        if !inflight {
            self.consume_mail_outbox(guid);
        }
    }

    pub(crate) fn apply_mail_delivery_finished(
        &mut self,
        guid: u32,
        ok: bool,
        appended: Vec<(i32, i32, u16)>,
    ) {
        let Some(outbox) = self.mail_outbox.get_mut(&guid) else {
            return;
        };
        if ok {
            outbox.fail_count = 0;
            outbox.last_appended.extend(appended);
            for p in &mut outbox.pending {
                if p.persist == MailPersistState::InFlight {
                    p.persist = MailPersistState::Acked;
                }
            }
        } else {
            outbox.fail_count = outbox.fail_count.saturating_add(1);
            for p in &mut outbox.pending {
                if p.persist == MailPersistState::InFlight {
                    p.persist = MailPersistState::Queued;
                }
            }
            tracing::warn!(guid, fails = outbox.fail_count, "offline mail DB append failed");
        }
        self.spawn_mail_append(guid);
    }

    /// After an ack, finish a deferred login if nothing is still in flight.
    pub(crate) fn take_deferred_login_if_mail_ready(
        &mut self,
        guid: u32,
    ) -> Option<DeferredLogin> {
        if self.mail_login_should_defer(guid) {
            return None;
        }
        self.mail_deferred_login.remove(&guid)
    }

    fn spawn_mail_append(&mut self, guid: u32) {
        let Some(sched) = self.scheduler.as_ref() else {
            return;
        };
        let Ok(handle) = tokio::runtime::Handle::try_current() else {
            return;
        };
        let Some(outbox) = self.mail_outbox.get_mut(&guid) else {
            return;
        };
        if outbox.has_inflight() || outbox.fail_count >= MAX_APPEND_FAILS {
            return;
        }
        let mut batch = Vec::new();
        let mut any = false;
        for p in &mut outbox.pending {
            if p.persist == MailPersistState::Queued && !p.delivered_live {
                crate::depot_append::append_offset_records(&mut batch, p.records.clone());
                p.persist = MailPersistState::InFlight;
                any = true;
            }
        }
        if !any {
            return;
        }
        let tx = sched.ctrl_sender();
        let db = self.db.clone();
        handle.spawn(async move {
            let store = ItemStore::new(&db);
            let mut rows = store
                .load_items(guid as i32, ItemTable::Depot)
                .await
                .unwrap_or_default();
            let start = rows.len();
            crate::depot_append::append_offset_records(&mut rows, batch);
            let appended: Vec<(i32, i32, u16)> = rows[start..]
                .iter()
                .map(|r| (r.pid, r.sid, r.itemtype))
                .collect();
            let ok = store
                .save_items(guid as i32, ItemTable::Depot, &rows)
                .await
                .is_ok();
            let appended = if ok { appended } else { Vec::new() };
            let _ = tx.send(GameCommand::MailDeliveryFinished {
                guid,
                ok,
                appended,
            });
        });
    }
}
