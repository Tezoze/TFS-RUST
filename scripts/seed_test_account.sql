-- Test account for local dev — matches login: account `1` / password `1`, character `Test`.
-- Password is SHA1 hex of plain text `1` (TFS `transformToSHA1` / `tfs-rust-db::sha1_password_hex`).
-- Temple spawn: Thais (town_id 1) on the default Forgotten Server map.

DELETE FROM `players` WHERE `name` = 'Test';
DELETE FROM `accounts` WHERE `name` = '1';

INSERT INTO `accounts` (`name`, `password`, `type`, `premium_ends_at`, `email`, `creation`)
VALUES ('1', '356a192b7913b04c54574d18c28d46e6395428ab', 1, 0, '', UNIX_TIMESTAMP());

SET @account_id = LAST_INSERT_ID();

INSERT INTO `players` (
  `name`, `group_id`, `account_id`, `level`, `vocation`,
  `health`, `healthmax`, `experience`,
  `lookbody`, `lookfeet`, `lookhead`, `looklegs`, `looktype`, `lookaddons`,
  `direction`, `maglevel`, `mana`, `manamax`,
  `soul`, `town_id`, `posx`, `posy`, `posz`,
  `cap`, `sex`
) VALUES (
  'Test', 1, @account_id, 1, 0,
  150, 150, 0,
  0, 0, 0, 0, 136, 0,
  2, 0, 0, 0,
  100, 1, 32369, 32241, 7,
  400, 0
);
