-- Test account for bcrypt upgrade smoke test — login: account `2` / password `2`, character `Two`.
-- Password is SHA1 hex of plain text `2` (TFS `transformToSHA1` / `tfs_rust_db::sha1_password_hex`).
-- First successful login upgrades this row to bcrypt (`legacySha1Enabled` in config.lua).
-- Temple spawn: Thais (town_id 1) on the default Forgotten Server map.

DELETE FROM `players` WHERE `name` = 'Two';
DELETE FROM `accounts` WHERE `name` = '2';

INSERT INTO `accounts` (`name`, `password`, `type`, `premium_ends_at`, `email`, `creation`)
VALUES ('2', 'da4b9237bacccdf19c0760cab7aec4a8359010b0', 1, 0, '', UNIX_TIMESTAMP());

SET @account_id = LAST_INSERT_ID();

INSERT INTO `players` (
  `name`, `group_id`, `account_id`, `level`, `vocation`,
  `health`, `healthmax`, `experience`,
  `lookbody`, `lookfeet`, `lookhead`, `looklegs`, `looktype`, `lookaddons`,
  `direction`, `maglevel`, `mana`, `manamax`,
  `soul`, `town_id`, `posx`, `posy`, `posz`,
  `cap`, `sex`
) VALUES (
  'Two', 1, @account_id, 1, 0,
  150, 150, 0,
  0, 0, 0, 0, 136, 0,
  2, 0, 0, 0,
  100, 1, 32369, 32241, 7,
  400, 0
);
