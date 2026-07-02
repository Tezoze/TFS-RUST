-- 772 `SKILL_FED` persistence — food counter + regen interval.
-- C++ reference: `crskill.cc:220` `TimerValue` (Cycle = food_remaining),
-- `crskill.cc:19-24` `Get` (Act = food_level = regen interval),
-- `crplayer.cc:2496` save Cycle, `crplayer.cc:2241` load Cycle.
-- TVP/TFS-1.4.2 do not track food (condition-only regen); these columns are 772-only.
ALTER TABLE `players`
  ADD COLUMN `food_remaining` int NOT NULL DEFAULT 0,
  ADD COLUMN `food_level` int NOT NULL DEFAULT 0;
