-- Death metadata: kill_statistics table + soul timer columns on players.
-- Pack: TFS `kill_statistics` (`data/migrations/1.lua`); `/deathlist` uses `player_deaths`.
-- Corpus: `WriteKillStatistics` — `crmain.cc`; `TSkill` Cycle/Count/MaxCount — `crskill.cc`.
--
-- Fresh SQLx installs: this CREATE TABLE uses PRIMARY KEY(name) so
-- ON DUPLICATE KEY UPDATE merge works.
-- Existing DBs that already have the Lua table (KEY(name) not UNIQUE/PK):
-- CREATE TABLE IF NOT EXISTS is a no-op and ON DUPLICATE KEY will not fire.
-- Prefer UNIQUE/PK(name) on those shards if kill-stat merge is required.

CREATE TABLE IF NOT EXISTS `kill_statistics` (
  `name` varchar(35) NOT NULL,
  `killed_by` int unsigned NOT NULL DEFAULT 0,
  `killed` int unsigned NOT NULL DEFAULT 0,
  `time` int unsigned NOT NULL DEFAULT 0,
  PRIMARY KEY (`name`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8;

ALTER TABLE `players`
  ADD COLUMN `soul_cycle` int NOT NULL DEFAULT 0,
  ADD COLUMN `soul_count` int NOT NULL DEFAULT 0,
  ADD COLUMN `soul_max_count` int NOT NULL DEFAULT 0;
