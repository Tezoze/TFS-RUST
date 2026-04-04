-- Arito's Task - Cave Teleporter
-- Only usable after starting Arito's Task (storage >= 1)

local teleporterPos = Position(33205, 32531, 7)
local destinationPos = Position(33205, 32530, 8)

local aritosTaskTeleporter = MoveEvent()

function aritosTaskTeleporter.onStepIn(creature, item, position, fromPosition)
	local player = creature:getPlayer()
	if not player then
		return true
	end
	
	-- Check if player has started Arito's Task
	if player:getStorageValue(Storage.TibiaTales.AritosTask) < 1 then
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You cannot pass through this portal.")
		player:teleportTo(fromPosition, true)
		fromPosition:sendMagicEffect(CONST_ME_TELEPORT)
		return false
	end
	
	player:teleportTo(destinationPos)
	destinationPos:sendMagicEffect(CONST_ME_TELEPORT)
	return true
end

aritosTaskTeleporter:position(teleporterPos)
aritosTaskTeleporter:register()
