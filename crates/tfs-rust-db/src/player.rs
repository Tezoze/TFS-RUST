//! Player load/save aligned with TFS `IOLoginData::loadPlayer` / `savePlayer`.
// C++ reference: src/iologindata.cpp IOLoginData::{loadPlayer, savePlayer}
//
// SQL types: `schema.sql` / `migrations/20240101000000_tfs_142_baseline.sql` (`players`, `guild_membership`, …).

use crate::items::{ItemRecord, ItemStore, ItemTable};
use crate::pool::DbPool;
use sqlx::FromRow;
use tfs_rust_common::error::{Result, TfsRustError};

/// Full `players` row — Rust types match TFS 1.4.2 MariaDB column signedness/size.
#[derive(Debug, Clone, FromRow)]
pub struct PlayerRecord {
    pub id: i32,
    pub name: String,
    pub account_id: i32,
    pub group_id: i32,
    pub sex: i32,
    pub vocation: i32,
    pub experience: u64,
    pub level: i32,
    pub maglevel: i32,
    pub health: i32,
    pub healthmax: i32,
    pub blessings: i8,
    pub mana: i32,
    pub manamax: i32,
    pub manaspent: u64,
    pub soul: u32,
    pub lookbody: i32,
    pub lookfeet: i32,
    pub lookhead: i32,
    pub looklegs: i32,
    pub looktype: i32,
    pub lookaddons: i32,
    pub posx: i32,
    pub posy: i32,
    pub posz: i32,
    pub cap: i32,
    pub lastlogin: u64,
    pub lastlogout: u64,
    pub lastip: u32,
    pub conditions: Option<Vec<u8>>,
    pub skulltime: i64,
    pub skull: i8,
    pub town_id: i32,
    pub balance: u64,
    pub offlinetraining_time: u16,
    pub offlinetraining_skill: i32,
    pub stamina: u16,
    pub skill_fist: u32,
    pub skill_fist_tries: u64,
    pub skill_club: u32,
    pub skill_club_tries: u64,
    pub skill_sword: u32,
    pub skill_sword_tries: u64,
    pub skill_axe: u32,
    pub skill_axe_tries: u64,
    pub skill_dist: u32,
    pub skill_dist_tries: u64,
    pub skill_shielding: u32,
    pub skill_shielding_tries: u64,
    pub skill_fishing: u32,
    pub skill_fishing_tries: u64,
    pub direction: u8,
    pub save: i8,
    pub onlinetime: i64,
    pub deletion: i64,
}

#[derive(Debug, Clone, FromRow)]
pub struct GuildMembershipRow {
    pub guild_id: i32,
    pub rank_id: i32,
    pub nick: String,
}

#[derive(Debug, Clone, Default)]
pub struct PlayerItemPayload {
    pub inventory: Vec<ItemRecord>,
    pub depot: Vec<ItemRecord>,
    pub inbox: Vec<ItemRecord>,
    pub store_inbox: Vec<ItemRecord>,
}

/// One row from `account_viplist` + friend name (`IOLoginData::getVIPEntries`).
#[derive(Debug, Clone)]
pub struct VipEntry {
    pub player_id: u32,
    pub name: String,
    pub description: String,
    pub icon: u32,
    pub notify: bool,
}

/// Everything `IOLoginData::loadPlayer` pulls from SQL for one character.
#[derive(Debug, Clone)]
pub struct LoadedPlayerData {
    pub player: PlayerRecord,
    /// `accounts.premium_ends_at` (unix seconds); used for `0x9F` basic data / premium checks.
    pub premium_ends_at: u32,
    pub spells: Vec<String>,
    pub storage: Vec<(u32, i32)>,
    /// VIP list for `sendVIPEntries` / opcode `0xD2`.
    pub vip_list: Vec<VipEntry>,
    pub guild: Option<GuildMembershipRow>,
    pub items: PlayerItemPayload,
}

/// Payload for `IOLoginData::savePlayer` (guild/VIP are not written here — same as C++).
#[derive(Debug, Clone)]
pub struct PlayerSaveData {
    pub player: PlayerRecord,
    pub spells: Vec<String>,
    pub items: PlayerItemPayload,
    pub storage: Vec<(u32, i32)>,
    /// When `true`, skip depot `DELETE`/`INSERT` like `lastDepotId == -1` in C++.
    pub skip_depot_save: bool,
}

pub struct PlayerStore<'a> {
    pool: &'a DbPool,
}

impl<'a> PlayerStore<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    /// Full load — C++ `IOLoginData::loadPlayer` (DB segments only; no `Player` object).
    pub async fn load_player_full(&self, name: &str) -> Result<Option<LoadedPlayerData>> {
        const SQL: &str = r#"SELECT id, name, account_id, group_id, sex, vocation, experience, level, maglevel,
            health, healthmax, blessings, mana, manamax, manaspent, soul,
            lookbody, lookfeet, lookhead, looklegs, looktype, lookaddons,
            posx, posy, posz, cap, lastlogin, lastlogout, lastip, conditions,
            skulltime, skull, town_id, balance, offlinetraining_time, offlinetraining_skill, stamina,
            skill_fist, skill_fist_tries, skill_club, skill_club_tries, skill_sword, skill_sword_tries,
            skill_axe, skill_axe_tries, skill_dist, skill_dist_tries, skill_shielding, skill_shielding_tries,
            skill_fishing, skill_fishing_tries, direction, save, onlinetime, deletion
            FROM players WHERE name = ? AND deletion = 0"#;

        let Some(player) = self
            .pool
            .execute_with_retry(|| {
                let pool = self.pool.inner().clone();
                let name = name.to_string();
                async move {
                    sqlx::query_as::<_, PlayerRecord>(SQL)
                        .bind(&name)
                        .fetch_optional(&pool)
                        .await
                }
            })
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?
        else {
            return Ok(None);
        };

        let pid = player.id;
        let account_id = player.account_id;

        let premium_ends_at: u32 = self
            .pool
            .execute_with_retry(|| {
                let pool = self.pool.inner().clone();
                async move {
                    // `accounts.premium_ends_at` is `INT UNSIGNED` in TFS 1.4.2 — not `BIGINT`.
                    let v: Option<u32> = sqlx::query_scalar(
                        "SELECT premium_ends_at FROM accounts WHERE id = ?",
                    )
                    .bind(account_id)
                    .fetch_one(&pool)
                    .await?;
                    Ok(v.unwrap_or(0))
                }
            })
            .await?;

        let spells = self.load_spells(pid).await?;
        let storage = self.load_storage(pid).await?;
        let vip_list = self.load_vip_entries(account_id).await?;
        let guild = self.load_guild(pid).await?;

        let items = PlayerItemPayload {
            inventory: ItemStore::new(self.pool)
                .load_items(pid, ItemTable::Inventory)
                .await?,
            depot: ItemStore::new(self.pool)
                .load_items(pid, ItemTable::Depot)
                .await?,
            inbox: ItemStore::new(self.pool)
                .load_items(pid, ItemTable::Inbox)
                .await?,
            store_inbox: ItemStore::new(self.pool)
                .load_items(pid, ItemTable::StoreInbox)
                .await?,
        };

        Ok(Some(LoadedPlayerData {
            player,
            premium_ends_at,
            spells,
            storage,
            vip_list,
            guild,
            items,
        }))
    }

    async fn load_spells(&self, player_id: i32) -> Result<Vec<String>> {
        self.pool
            .execute_with_retry(|| {
                let pool = self.pool.inner().clone();
                async move {
                    sqlx::query_scalar::<_, String>(
                        "SELECT name FROM player_spells WHERE player_id = ?",
                    )
                    .bind(player_id)
                    .fetch_all(&pool)
                    .await
                }
            })
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))
    }

    async fn load_storage(&self, player_id: i32) -> Result<Vec<(u32, i32)>> {
        #[derive(FromRow)]
        struct Row {
            key: u32,
            value: i32,
        }
        let rows = self
            .pool
            .execute_with_retry(|| {
                let pool = self.pool.inner().clone();
                async move {
                    sqlx::query_as::<_, Row>(
                        "SELECT `key`, `value` FROM player_storage WHERE player_id = ?",
                    )
                    .bind(player_id)
                    .fetch_all(&pool)
                    .await
                }
            })
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;
        Ok(rows.into_iter().map(|r| (r.key, r.value)).collect())
    }

    async fn load_vip_entries(&self, account_id: i32) -> Result<Vec<VipEntry>> {
        #[derive(FromRow)]
        struct Row {
            player_id: i32,
            name: Option<String>,
            description: String,
            icon: i32,
            notify: i8,
        }
        let rows: Vec<Row> = self
            .pool
            .execute_with_retry(|| {
                let pool = self.pool.inner().clone();
                async move {
                    sqlx::query_as::<_, Row>(
                        r#"SELECT v.player_id, p.name, v.description, v.icon, v.notify
                           FROM account_viplist v
                           INNER JOIN players p ON p.id = v.player_id
                           WHERE v.account_id = ?"#,
                    )
                    .bind(account_id)
                    .fetch_all(&pool)
                    .await
                }
            })
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;
        Ok(rows
            .into_iter()
            .map(|r| VipEntry {
                player_id: r.player_id as u32,
                name: r.name.unwrap_or_default(),
                description: r.description,
                icon: r.icon.max(0) as u32,
                notify: r.notify != 0,
            })
            .collect())
    }

    async fn load_guild(&self, player_id: i32) -> Result<Option<GuildMembershipRow>> {
        self.pool
            .execute_with_retry(|| {
                let pool = self.pool.inner().clone();
                async move {
                    sqlx::query_as::<_, GuildMembershipRow>(
                        "SELECT guild_id, rank_id, nick FROM guild_membership WHERE player_id = ?",
                    )
                    .bind(player_id)
                    .fetch_optional(&pool)
                    .await
                }
            })
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))
    }

    /// C++ `IOLoginData::savePlayer` — transactional; does not retry the whole transaction (retry outer caller if needed).
    pub async fn save_player(&self, data: &PlayerSaveData) -> Result<()> {
        if data.player.save == 0 {
            return self
                .save_login_only(data.player.id, data.player.lastlogin, data.player.lastip)
                .await;
        }

        let mut tx = self
            .pool
            .inner()
            .begin()
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;

        let p = &data.player;
        sqlx::query(
            r#"UPDATE players SET
            level = ?, group_id = ?, vocation = ?, health = ?, healthmax = ?, experience = ?,
            lookbody = ?, lookfeet = ?, lookhead = ?, looklegs = ?, looktype = ?, lookaddons = ?,
            maglevel = ?, mana = ?, manamax = ?, manaspent = ?, soul = ?, town_id = ?,
            posx = ?, posy = ?, posz = ?, cap = ?, sex = ?, lastlogin = ?, lastip = ?, conditions = ?,
            skulltime = ?, skull = ?, lastlogout = ?, balance = ?, offlinetraining_time = ?, offlinetraining_skill = ?,
            stamina = ?, skill_fist = ?, skill_fist_tries = ?, skill_club = ?, skill_club_tries = ?,
            skill_sword = ?, skill_sword_tries = ?, skill_axe = ?, skill_axe_tries = ?,
            skill_dist = ?, skill_dist_tries = ?, skill_shielding = ?, skill_shielding_tries = ?,
            skill_fishing = ?, skill_fishing_tries = ?, direction = ?, onlinetime = ?, blessings = ?
            WHERE id = ?"#,
        )
        .bind(p.level)
        .bind(p.group_id)
        .bind(p.vocation)
        .bind(p.health)
        .bind(p.healthmax)
        .bind(p.experience)
        .bind(p.lookbody)
        .bind(p.lookfeet)
        .bind(p.lookhead)
        .bind(p.looklegs)
        .bind(p.looktype)
        .bind(p.lookaddons)
        .bind(p.maglevel)
        .bind(p.mana)
        .bind(p.manamax)
        .bind(p.manaspent)
        .bind(p.soul)
        .bind(p.town_id)
        .bind(p.posx)
        .bind(p.posy)
        .bind(p.posz)
        .bind(p.cap)
        .bind(p.sex)
        .bind(p.lastlogin)
        .bind(p.lastip)
        .bind(p.conditions.as_deref())
        .bind(p.skulltime)
        .bind(p.skull)
        .bind(p.lastlogout)
        .bind(p.balance)
        .bind(p.offlinetraining_time)
        .bind(p.offlinetraining_skill)
        .bind(p.stamina)
        .bind(p.skill_fist)
        .bind(p.skill_fist_tries)
        .bind(p.skill_club)
        .bind(p.skill_club_tries)
        .bind(p.skill_sword)
        .bind(p.skill_sword_tries)
        .bind(p.skill_axe)
        .bind(p.skill_axe_tries)
        .bind(p.skill_dist)
        .bind(p.skill_dist_tries)
        .bind(p.skill_shielding)
        .bind(p.skill_shielding_tries)
        .bind(p.skill_fishing)
        .bind(p.skill_fishing_tries)
        .bind(p.direction)
        .bind(p.onlinetime)
        .bind(p.blessings)
        .bind(p.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| TfsRustError::Database(e.to_string()))?;

        sqlx::query("DELETE FROM player_spells WHERE player_id = ?")
            .bind(p.id)
            .execute(&mut *tx)
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;

        for spell in &data.spells {
            sqlx::query("INSERT INTO player_spells (player_id, name) VALUES (?, ?)")
                .bind(p.id)
                .bind(spell)
                .execute(&mut *tx)
                .await
                .map_err(|e| TfsRustError::Database(e.to_string()))?;
        }

        ItemStore::save_items_tx(&mut tx, p.id, ItemTable::Inventory, &data.items.inventory)
            .await?;

        if !data.skip_depot_save {
            ItemStore::save_items_tx(&mut tx, p.id, ItemTable::Depot, &data.items.depot).await?;
        }

        ItemStore::save_items_tx(&mut tx, p.id, ItemTable::Inbox, &data.items.inbox).await?;
        ItemStore::save_items_tx(
            &mut tx,
            p.id,
            ItemTable::StoreInbox,
            &data.items.store_inbox,
        )
        .await?;

        sqlx::query("DELETE FROM player_storage WHERE player_id = ?")
            .bind(p.id)
            .execute(&mut *tx)
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;

        for (k, v) in &data.storage {
            sqlx::query("INSERT INTO player_storage (player_id, `key`, `value`) VALUES (?, ?, ?)")
                .bind(p.id)
                .bind(k)
                .bind(v)
                .execute(&mut *tx)
                .await
                .map_err(|e| TfsRustError::Database(e.to_string()))?;
        }

        tx.commit()
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;
        Ok(())
    }

    /// C++ branch when `save == 0`: only `lastlogin` + `lastip`.
    async fn save_login_only(&self, id: i32, lastlogin: u64, lastip: u32) -> Result<()> {
        self.pool
            .execute_with_retry(|| {
                let pool = self.pool.inner().clone();
                async move {
                    sqlx::query("UPDATE players SET lastlogin = ?, lastip = ? WHERE id = ?")
                        .bind(lastlogin)
                        .bind(lastip)
                        .bind(id)
                        .execute(&pool)
                        .await
                }
            })
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;
        Ok(())
    }
}
