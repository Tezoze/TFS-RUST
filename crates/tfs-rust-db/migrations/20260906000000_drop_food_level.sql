-- Drop invented `players.food_level` (mistaken SKILL_FED Act / regen interval).
-- Hunger remains `food_remaining`. Item regen cadence is equipped DAct, not persisted.
-- C++: `crskill.cc:195-204` SetTimer never writes Act; `crmain.cc:1087` Get() is DAct.

ALTER TABLE `players` DROP COLUMN `food_level`;
