local config = {
	arenaPosition = Position(33154, 31415, 7),
	successPosition = Position(33145, 31419, 7),
	exitPosition = Position(33153, 31420, 7) -- Position to teleport failed players
}

local function completeTest(cid)
	local player = Player(cid)
	if not player then
		return true
	end

	-- Check if player still has the "in arena" questline value (19)
	-- Don't check exact position - player moves around during the fight
	if player:getStorageValue(Storage.TheNewFrontier.Questline) == 19 then
		player:teleportTo(config.successPosition)
		config.successPosition:sendMagicEffect(CONST_ME_TELEPORT)
		player:say('You have passed the test. Report to Curos.', TALKTYPE_MONSTER_SAY)
	end
end

function onStepIn(creature, item, position, fromPosition)
	local player = creature:getPlayer()
	if not player then
		return true
	end

	if player:getStorageValue(Storage.TheNewFrontier.Questline) ~= 18 then
		player:teleportTo(fromPosition)
		fromPosition:sendMagicEffect(CONST_ME_TELEPORT)
		player:sendTextMessage(MESSAGE_STATUS_SMALL, 'You don\'t have access to this area.')
		return true
	end

	player:setStorageValue(Storage.TheNewFrontier.Questline, 19)
	player:teleportTo(config.arenaPosition)
	config.arenaPosition:sendMagicEffect(CONST_ME_TELEPORT)
	player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have entered the arena. Survive for 2 minutes!')
	addEvent(completeTest, 2 * 60 * 1000, player.uid)
	return true
end

-- onStepOut is no longer needed - the teleport tile shouldn't reset the quest
-- The player can only leave by:
-- 1. Surviving 2 minutes (completeTest teleports them out)
-- 2. Dying (handled by death event if needed)
-- 3. Logging out (quest stays at 19, they can continue when they log back in)
