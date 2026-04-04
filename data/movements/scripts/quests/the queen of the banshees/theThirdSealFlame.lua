local config = {
	[0] = 50015,
	[1] = 50016,
	[2] = 50017,
	[3] = 50018,
	[4] = 50019,

	basinPositions = {
		Position(32214, 31850, 15),
		Position(32215, 31850, 15),
		Position(32216, 31850, 15)
	},

	switchPositions = {
		Position(32220, 31846, 15),
		Position(32220, 31845, 15),
		Position(32220, 31844, 15),
		Position(32220, 31843, 15),
		Position(32220, 31842, 15)
	},

	destination = Position(32271, 31857, 15)
}

local function resetItem(position, itemId, transformId)
	local item = Tile(position):getItemById(itemId)
	if item then
		item:transform(transformId)
	end
end

function onStepIn(creature, item, position, fromPosition)
	local player = creature:getPlayer()
	if not player then
		return true
	end

	-- print(string.format("[ThirdSeal Flame Debug] Player %s stepped on flame - VERSION 2.0", player:getName()))

	-- Check if player already completed the quest
	if player:getStorageValue(Storage.QueenOfBansheesQuest.ThirdSeal) >= 1 then
		player:sendCancelMessage('You have already completed this seal.')
		player:teleportTo(fromPosition)
		fromPosition:sendMagicEffect(CONST_ME_TELEPORT)
		return true
	end

	-- Check if the mystic flame is active (set when all levers are completed)
	local flameActive = Game.getStorageValue(Storage.QueenOfBansheesQuest.ThirdSealActive) or 0

	-- print(string.format("[ThirdSeal Flame Debug] Flame active: %s", flameActive == 1 and "YES" or "NO"))

	if flameActive < 1 then
		player:sendCancelMessage('The mystic flame is not yet active.')
		player:teleportTo(fromPosition)
		fromPosition:sendMagicEffect(CONST_ME_TELEPORT)
		return true
	end

	-- Complete the Third Seal for this individual player
	player:setStorageValue(Storage.QueenOfBansheesQuest.ThirdSeal, 1)
	player:teleportTo(config.destination)
	config.destination:sendMagicEffect(CONST_ME_TELEPORT)
	-- player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have completed the Third Seal of the Queen of the Banshees Quest!')

	-- print("[ThirdSeal Flame Debug] Player completed their individual quest - flame remains active for other players")

	return true
end
