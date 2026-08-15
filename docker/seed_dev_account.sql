-- Dev login for Docker Compose. Idempotent (safe on every `compose up`).
-- Client: account `1`, password `1`.
-- Characters: God, Master Sorcerer, Elder Druid, Royal Paladin, Elite Knight.
-- Password is SHA1 hex of `1` (TFS `transformToSHA1`); first login upgrades to bcrypt.
-- Town + spawn: Thais temple (town_id 1, 32369,32241,7 — `the_oracle.lua` / `thais_home.lua`).
-- Exp for level 100 = experience_for_level_poly(100, 100).

-- `type` 6 = ACCOUNT_TYPE_GOD. Group 6 is not enough: `/m` also checks account type.
INSERT INTO `accounts` (`name`, `password`, `type`, `premium_ends_at`, `email`, `creation`)
VALUES ('1', '356a192b7913b04c54574d18c28d46e6395428ab', 6, 0, '', UNIX_TIMESTAMP())
ON DUPLICATE KEY UPDATE `type` = 6;

-- God: group 6 (`data/defs/groups.lua`). Voc none. Full flags on login.
INSERT INTO `players` (
  `name`, `group_id`, `account_id`, `level`, `vocation`,
  `health`, `healthmax`, `experience`,
  `lookbody`, `lookfeet`, `lookhead`, `looklegs`, `looktype`, `lookaddons`,
  `direction`, `maglevel`, `mana`, `manamax`,
  `soul`, `town_id`, `posx`, `posy`, `posz`,
  `cap`, `sex`,
  `skill_fist`, `skill_club`, `skill_sword`, `skill_axe`, `skill_dist`, `skill_shielding`, `skill_fishing`
)
SELECT
  'God', 6, a.`id`, 100, 0,
  10000, 10000, 15694800,
  94, 94, 94, 94, 75, 0,
  2, 60, 10000, 10000,
  200, 1, 32369, 32241, 7,
  10000, 1,
  100, 100, 100, 100, 100, 100, 10
FROM `accounts` a
WHERE a.`name` = '1'
  AND NOT EXISTS (SELECT 1 FROM `players` p WHERE p.`name` = 'God');

-- Master Sorcerer voc 5: HP 150+5*99=645, mana 30*99=2970, cap 400+10*99=1390
INSERT INTO `players` (
  `name`, `group_id`, `account_id`, `level`, `vocation`,
  `health`, `healthmax`, `experience`,
  `lookbody`, `lookfeet`, `lookhead`, `looklegs`, `looktype`, `lookaddons`,
  `direction`, `maglevel`, `mana`, `manamax`,
  `soul`, `town_id`, `posx`, `posy`, `posz`,
  `cap`, `sex`,
  `skill_fist`, `skill_club`, `skill_sword`, `skill_axe`, `skill_dist`, `skill_shielding`, `skill_fishing`
)
SELECT
  'Master Sorcerer', 1, a.`id`, 100, 5,
  645, 645, 15694800,
  86, 86, 114, 114, 130, 0,
  2, 60, 2970, 2970,
  200, 1, 32369, 32241, 7,
  1390, 1,
  10, 10, 10, 10, 10, 10, 10
FROM `accounts` a
WHERE a.`name` = '1'
  AND NOT EXISTS (SELECT 1 FROM `players` p WHERE p.`name` = 'Master Sorcerer');

-- Elder Druid voc 6: same vitals as MS
INSERT INTO `players` (
  `name`, `group_id`, `account_id`, `level`, `vocation`,
  `health`, `healthmax`, `experience`,
  `lookbody`, `lookfeet`, `lookhead`, `looklegs`, `looktype`, `lookaddons`,
  `direction`, `maglevel`, `mana`, `manamax`,
  `soul`, `town_id`, `posx`, `posy`, `posz`,
  `cap`, `sex`,
  `skill_fist`, `skill_club`, `skill_sword`, `skill_axe`, `skill_dist`, `skill_shielding`, `skill_fishing`
)
SELECT
  'Elder Druid', 1, a.`id`, 100, 6,
  645, 645, 15694800,
  69, 76, 69, 88, 130, 0,
  2, 60, 2970, 2970,
  200, 1, 32369, 32241, 7,
  1390, 1,
  10, 10, 10, 10, 10, 10, 10
FROM `accounts` a
WHERE a.`name` = '1'
  AND NOT EXISTS (SELECT 1 FROM `players` p WHERE p.`name` = 'Elder Druid');

-- Royal Paladin voc 7: HP 150+10*99=1140, mana 15*99=1485, cap 400+20*99=2380
INSERT INTO `players` (
  `name`, `group_id`, `account_id`, `level`, `vocation`,
  `health`, `healthmax`, `experience`,
  `lookbody`, `lookfeet`, `lookhead`, `looklegs`, `looktype`, `lookaddons`,
  `direction`, `maglevel`, `mana`, `manamax`,
  `soul`, `town_id`, `posx`, `posy`, `posz`,
  `cap`, `sex`,
  `skill_fist`, `skill_club`, `skill_sword`, `skill_axe`, `skill_dist`, `skill_shielding`, `skill_fishing`
)
SELECT
  'Royal Paladin', 1, a.`id`, 100, 7,
  1140, 1140, 15694800,
  94, 115, 0, 58, 129, 0,
  2, 16, 1485, 1485,
  200, 1, 32369, 32241, 7,
  2380, 1,
  10, 10, 10, 10, 100, 80, 10
FROM `accounts` a
WHERE a.`name` = '1'
  AND NOT EXISTS (SELECT 1 FROM `players` p WHERE p.`name` = 'Royal Paladin');

-- Elite Knight voc 8: HP 150+15*99=1635, mana 5*99=495, cap 400+25*99=2875
INSERT INTO `players` (
  `name`, `group_id`, `account_id`, `level`, `vocation`,
  `health`, `healthmax`, `experience`,
  `lookbody`, `lookfeet`, `lookhead`, `looklegs`, `looktype`, `lookaddons`,
  `direction`, `maglevel`, `mana`, `manamax`,
  `soul`, `town_id`, `posx`, `posy`, `posz`,
  `cap`, `sex`,
  `skill_fist`, `skill_club`, `skill_sword`, `skill_axe`, `skill_dist`, `skill_shielding`, `skill_fishing`
)
SELECT
  'Elite Knight', 1, a.`id`, 100, 8,
  1635, 1635, 15694800,
  95, 76, 78, 94, 131, 0,
  2, 6, 495, 495,
  200, 1, 32369, 32241, 7,
  2875, 1,
  10, 10, 10, 100, 10, 100, 10
FROM `accounts` a
WHERE a.`name` = '1'
  AND NOT EXISTS (SELECT 1 FROM `players` p WHERE p.`name` = 'Elite Knight');

-- Equipment: pid 1-10 = CONST_SLOT_*; backpack contents pid = backpack sid.
-- Slots: HEAD=1 NECK=2 BACKPACK=3 ARMOR=4 RIGHT=5 LEFT=6 LEGS=7 FEET=8 RING=9 AMMO=10
-- Item ids from data/items/items.xml (772-era pack). Empty attributes blob.

INSERT INTO `player_items` (`player_id`, `pid`, `sid`, `itemtype`, `count`, `attributes`)
SELECT p.`id`, v.`pid`, v.`sid`, v.`itemtype`, v.`count`, ''
FROM `players` p
INNER JOIN (
  SELECT 1 AS pid, 101 AS sid, 2471 AS itemtype, 1 AS `count` UNION ALL -- golden helmet
  SELECT 2, 102, 2171, 1 UNION ALL -- platinum amulet
  SELECT 3, 103, 1988, 1 UNION ALL -- backpack
  SELECT 4, 104, 2472, 1 UNION ALL -- magic plate armor
  SELECT 5, 105, 2514, 1 UNION ALL -- mastermind shield
  SELECT 6, 106, 2400, 1 UNION ALL -- magic sword
  SELECT 7, 107, 2470, 1 UNION ALL -- golden legs
  SELECT 8, 108, 2195, 1 UNION ALL -- boots of haste
  SELECT 9, 109, 2167, 1 UNION ALL -- energy ring
  SELECT 103, 111, 2268, 100 UNION ALL -- SD
  SELECT 103, 112, 2311, 100 UNION ALL -- HMM
  SELECT 103, 113, 2273, 100           -- UH
) v
WHERE p.`name` = 'God'
  AND NOT EXISTS (SELECT 1 FROM `player_items` i WHERE i.`player_id` = p.`id`);

INSERT INTO `player_items` (`player_id`, `pid`, `sid`, `itemtype`, `count`, `attributes`)
SELECT p.`id`, v.`pid`, v.`sid`, v.`itemtype`, v.`count`, ''
FROM `players` p
INNER JOIN (
  SELECT 1 AS pid, 101 AS sid, 2323 AS itemtype, 1 AS `count` UNION ALL -- hat of the mad
  SELECT 2, 102, 2171, 1 UNION ALL -- platinum amulet
  SELECT 3, 103, 1988, 1 UNION ALL -- backpack
  SELECT 4, 104, 2656, 1 UNION ALL -- blue robe
  SELECT 5, 105, 2175, 1 UNION ALL -- spellbook
  SELECT 6, 106, 2187, 1 UNION ALL -- wand of inferno
  SELECT 7, 107, 2649, 1 UNION ALL -- leather legs
  SELECT 8, 108, 2195, 1 UNION ALL -- boots of haste
  SELECT 9, 109, 2167, 1 UNION ALL -- energy ring
  SELECT 103, 111, 2268, 100 UNION ALL -- SD
  SELECT 103, 112, 2311, 100 UNION ALL -- HMM
  SELECT 103, 113, 2304, 100           -- GFB
) v
WHERE p.`name` = 'Master Sorcerer'
  AND NOT EXISTS (SELECT 1 FROM `player_items` i WHERE i.`player_id` = p.`id`);

INSERT INTO `player_items` (`player_id`, `pid`, `sid`, `itemtype`, `count`, `attributes`)
SELECT p.`id`, v.`pid`, v.`sid`, v.`itemtype`, v.`count`, ''
FROM `players` p
INNER JOIN (
  SELECT 1 AS pid, 101 AS sid, 2662 AS itemtype, 1 AS `count` UNION ALL -- magician hat
  SELECT 2, 102, 2171, 1 UNION ALL -- platinum amulet
  SELECT 3, 103, 1988, 1 UNION ALL -- backpack
  SELECT 4, 104, 2656, 1 UNION ALL -- blue robe
  SELECT 5, 105, 2175, 1 UNION ALL -- spellbook
  SELECT 6, 106, 2183, 1 UNION ALL -- tempest rod
  SELECT 7, 107, 2649, 1 UNION ALL -- leather legs
  SELECT 8, 108, 2195, 1 UNION ALL -- boots of haste
  SELECT 9, 109, 2167, 1 UNION ALL -- energy ring
  SELECT 103, 111, 2273, 100 UNION ALL -- UH
  SELECT 103, 112, 2274, 100 UNION ALL -- avalanche
  SELECT 103, 113, 2268, 100           -- SD (support)
) v
WHERE p.`name` = 'Elder Druid'
  AND NOT EXISTS (SELECT 1 FROM `player_items` i WHERE i.`player_id` = p.`id`);

INSERT INTO `player_items` (`player_id`, `pid`, `sid`, `itemtype`, `count`, `attributes`)
SELECT p.`id`, v.`pid`, v.`sid`, v.`itemtype`, v.`count`, ''
FROM `players` p
INNER JOIN (
  SELECT 1 AS pid, 101 AS sid, 2498 AS itemtype, 1 AS `count` UNION ALL -- royal helmet
  SELECT 2, 102, 2171, 1 UNION ALL -- platinum amulet
  SELECT 3, 103, 1988, 1 UNION ALL -- backpack
  SELECT 4, 104, 2486, 1 UNION ALL -- noble armor
  SELECT 6, 106, 2456, 1 UNION ALL -- bow (two-handed)
  SELECT 7, 107, 2647, 1 UNION ALL -- plate legs
  SELECT 8, 108, 2195, 1 UNION ALL -- boots of haste
  SELECT 9, 109, 2165, 1 UNION ALL -- stealth ring
  SELECT 10, 110, 2544, 100 UNION ALL -- arrows
  SELECT 103, 111, 2547, 100 UNION ALL -- power bolts
  SELECT 103, 112, 2273, 100 UNION ALL -- UH
  SELECT 103, 113, 2311, 100           -- HMM
) v
WHERE p.`name` = 'Royal Paladin'
  AND NOT EXISTS (SELECT 1 FROM `player_items` i WHERE i.`player_id` = p.`id`);

INSERT INTO `player_items` (`player_id`, `pid`, `sid`, `itemtype`, `count`, `attributes`)
SELECT p.`id`, v.`pid`, v.`sid`, v.`itemtype`, v.`count`, ''
FROM `players` p
INNER JOIN (
  SELECT 1 AS pid, 101 AS sid, 2493 AS itemtype, 1 AS `count` UNION ALL -- demon helmet
  SELECT 2, 102, 2171, 1 UNION ALL -- platinum amulet
  SELECT 3, 103, 1988, 1 UNION ALL -- backpack
  SELECT 4, 104, 2492, 1 UNION ALL -- dragon scale mail
  SELECT 5, 105, 2520, 1 UNION ALL -- demon shield
  SELECT 6, 106, 2432, 1 UNION ALL -- fire axe
  SELECT 7, 107, 2477, 1 UNION ALL -- knight legs
  SELECT 8, 108, 2645, 1 UNION ALL -- steel boots
  SELECT 9, 109, 2167, 1 UNION ALL -- energy ring
  SELECT 103, 111, 2273, 100           -- UH
) v
WHERE p.`name` = 'Elite Knight'
  AND NOT EXISTS (SELECT 1 FROM `player_items` i WHERE i.`player_id` = p.`id`);

-- Always pin town + login tile to Thais (covers characters created by an earlier seed).
UPDATE `players`
SET `town_id` = 1, `posx` = 32369, `posy` = 32241, `posz` = 7
WHERE `name` IN ('God', 'Master Sorcerer', 'Elder Druid', 'Royal Paladin', 'Elite Knight');
