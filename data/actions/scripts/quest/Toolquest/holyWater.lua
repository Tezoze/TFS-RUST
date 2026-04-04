local doorPosition = Position(32260, 32791, 7)
local shadowNexusPosition = Position(33115, 31702, 12)
local effectPositions = {
	Position(33113, 31702, 12),
	Position(33116, 31702, 12)
}

local function revertItem(position, itemId, transformId)
	local item = Tile(position):getItemById(itemId)
	if item then
		item:transform(transformId)
	end
end

local function nexusMessage(player, message)
	-- Send message to nearby players about shadow nexus damage
	local spectators = Game.getSpectators(shadowNexusPosition, false, true, 3, 3)
	for i = 1, #spectators do
		local spectator = spectators[i]
		if spectator:isPlayer() then
			spectator:say(message, TALKTYPE_MONSTER_SAY)
		end
	end
end

local config = {
	antler_talisman = 24664,
	sacred_antler_talisman = 24665
	}

function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	if not player or not item or not target or not target:isItem() then
		return false
	end

	local questlineStorage = Storage.TibiaTales.RestInHallowedGround.Questline
	local holyWaterStorage = Storage.TibiaTales.RestInHallowedGround.HolyWater

	if target.itemid == config.antler_talisman then
		item:transform(config.sacred_antler_talisman)
		item:remove(1)
		target:remove(1)
		-- Successfully transformed antler talisman
		player:getPosition():sendMagicEffect(CONST_ME_MAGIC_GREEN)
		return true
	end

	-- Eclipse
	if target.actionid == 2000 then
		item:remove(1)
		toPosition:sendMagicEffect(CONST_ME_FIREAREA)
		-- The Inquisition Questlog- 'Mission 2: Eclipse'
		player:setStorageValue(Storage.TheInquisition.Mission02, 2)
		player:setStorageValue(Storage.TheInquisition.Questline, 5)
		return true

	-- Haunted Ruin
	elseif target.actionid == 2003 then
		if player:getStorageValue(Storage.TheInquisition.Questline) ~= 12 then
			return true
		end

		Game.createMonster('Pirate Ghost', toPosition)
		item:remove(1)

		-- The Inquisition Questlog- 'Mission 4: The Haunted Ruin'
		player:setStorageValue(Storage.TheInquisition.Questline, 13)
		player:setStorageValue(Storage.TheInquisition.Mission04, 2)

		local doorItem = Tile(doorPosition):getItemById(8697)
		if doorItem then
			doorItem:transform(8696)
		end
		addEvent(revertItem, 10 * 1000, doorPosition, 8696, 8697)
		return true
	end

	-- Shadow Nexus
	local damageItems = {8753, 8755, 8757}
	local isDamageItem = false
	for _, itemId in ipairs(damageItems) do
		if target.itemid == itemId then
			isDamageItem = true
			break
		end
	end

	if isDamageItem then
		target:transform(target.itemid + 1)
		target:decay()
		nexusMessage(player, player:getName() .. ' damaged the shadow nexus! You can\'t damage it while it\'s burning.')
		shadowNexusPosition:sendMagicEffect(CONST_ME_HOLYAREA)

	elseif target.itemid == 8759 then
		if player:getStorageValue(Storage.TheInquisition.Questline) < 22 then
			-- The Inquisition Questlog- 'Mission 7: The Shadow Nexus'
			player:setStorageValue(Storage.TheInquisition.Mission07, 2)
			player:setStorageValue(Storage.TheInquisition.Questline, 22)
		end

		for i = 1, #effectPositions do
			effectPositions[i]:sendMagicEffect(CONST_ME_HOLYAREA)
		end

		nexusMessage(player, player:getName() .. ' destroyed the shadow nexus! In 20 seconds it will return to its original state.')
		item:remove(1)
	elseif target.actionid > 4007 and target.actionid < 4024 then

		local storages = {
			[4008] = Storage.TibiaTales.RestInHallowedGround.Graves.Grave1,
			[4009] = Storage.TibiaTales.RestInHallowedGround.Graves.Grave2,
			[4010] = Storage.TibiaTales.RestInHallowedGround.Graves.Grave3,
			[4011] = Storage.TibiaTales.RestInHallowedGround.Graves.Grave4,
			[4012] = Storage.TibiaTales.RestInHallowedGround.Graves.Grave5,
			[4013] = Storage.TibiaTales.RestInHallowedGround.Graves.Grave6,
			[4014] = Storage.TibiaTales.RestInHallowedGround.Graves.Grave7,
			[4015] = Storage.TibiaTales.RestInHallowedGround.Graves.Grave8,
			[4016] = Storage.TibiaTales.RestInHallowedGround.Graves.Grave9,
			[4017] = Storage.TibiaTales.RestInHallowedGround.Graves.Grave10,
			[4018] = Storage.TibiaTales.RestInHallowedGround.Graves.Grave11,
			[4019] = Storage.TibiaTales.RestInHallowedGround.Graves.Grave12,
			[4020] = Storage.TibiaTales.RestInHallowedGround.Graves.Grave13,
			[4021] = Storage.TibiaTales.RestInHallowedGround.Graves.Grave14,
			[4022] = Storage.TibiaTales.RestInHallowedGround.Graves.Grave15,
			[4023] = Storage.TibiaTales.RestInHallowedGround.Graves.Grave16
		}

		local graveStorage = storages[target.actionid]
		if not graveStorage then
			return false
		end

		if player:getStorageValue(graveStorage) == 1 then
			return true -- Grave already sanctified
		end

		if player:getStorageValue(questlineStorage) ~= 3 then
			return true -- Wrong quest stage
		end

		player:setStorageValue(graveStorage, 1)
		-- Successfully sanctified grave

		local cStorage = player:getStorageValue(holyWaterStorage)
		if cStorage < 16 then
			player:setStorageValue(holyWaterStorage, math.max(0, cStorage) + 1)
		elseif cStorage == 16 then
			player:setStorageValue(holyWaterStorage, -1)
			player:setStorageValue(questlineStorage, 4)
			item:transform(2006) -- Transform to empty vial
			-- Quest completed: all graves sanctified
		end

		toPosition:sendMagicEffect(CONST_ME_MAGIC_BLUE)
	end
	return true
end
