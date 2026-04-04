--[[
Requires database schema:

CREATE TABLE `global_storage`
(
    `key` INT UNSIGNED NOT NULL,
    `value` VARCHAR(255) NOT NULL DEFAULT '0',
    UNIQUE  (`key`)
) ENGINE = InnoDB;

]]

local function getGlobalStorageValueDB(key)
    local query = db.storeQuery("SELECT `value` FROM `global_storage` WHERE `key` = " .. key .. ";")
    if query ~= false then
        local val = result.getDataInt(query, "value")
        result.free(query)
        return val
    end
    return -1
end

local function setGlobalStorageValueDB(key, value)
    db.query("INSERT INTO `global_storage` (`key`, `value`) VALUES (".. key ..", ".. value ..") ON DUPLICATE KEY UPDATE `value` = ".. tonumber(value))
end

function onThink()
    if (tonumber(os.date("%d")) ~= getGlobalStorageValueDB(23456)) then
        setGlobalStorageValueDB(23456, (tonumber(os.date("%d"))))
        db.query("UPDATE `players` SET `onlinetime7`=`onlinetime6`, `onlinetime6`=`onlinetime5`, `onlinetime5`=`onlinetime4`, `onlinetime4`=`onlinetime3`, `onlinetime3`=`onlinetime2`, `onlinetime2`=`onlinetime1`, `onlinetime1`=`onlinetimetoday`, `onlinetimetoday`=0;")
        db.query("UPDATE `players` SET `exphist7`=`exphist6`, `exphist6`=`exphist5`, `exphist5`=`exphist4`, `exphist4`=`exphist3`, `exphist3`=`exphist2`, `exphist2`=`exphist1`, `exphist1`=IF(`experience`>=`exphist_lastexp`, `experience`-`exphist_lastexp`, 0), `exphist_lastexp`=`experience`;")
    end
    
    db.query("UPDATE `players` INNER JOIN `players_online` ON `players`.`id` = `players_online`.`player_id` SET `onlinetimetoday`=`onlinetimetoday`+60, `onlinetimeall`=`onlinetimeall`+60")

    return true
end