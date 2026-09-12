-- TVP-only tables missing from TFS schema.sql. Safe on the shared TFS DB
-- (CREATE IF NOT EXISTS). Login SELECT `player_murders`; raids.cpp uses `raids`.

CREATE TABLE IF NOT EXISTS `player_murders` (
  `id` int NOT NULL AUTO_INCREMENT,
  `player_id` int NOT NULL,
  `date` bigint NOT NULL DEFAULT '0',
  PRIMARY KEY (`id`),
  KEY `player_id` (`player_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8;

CREATE TABLE IF NOT EXISTS `raids` (
  `name` varchar(255) NOT NULL,
  `date` bigint NOT NULL DEFAULT '0',
  `count` int NOT NULL DEFAULT '0',
  UNIQUE KEY `nameindex` (`name`)
) ENGINE=InnoDB DEFAULT CHARSET=latin1;

-- TVP `getStream("conditions")` does not accept SQL NULL.
UPDATE `players` SET `conditions` = '' WHERE `conditions` IS NULL;
