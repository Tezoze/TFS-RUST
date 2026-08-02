-- 772 PlayerData::MurderTimestamps[20] — CSV of unix seconds (oldest→newest).
ALTER TABLE `players`
  ADD COLUMN `murder_timestamps` text NOT NULL DEFAULT '' AFTER `skulltime`;
