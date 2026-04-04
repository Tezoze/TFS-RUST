local teleportToPosition = Position(32780, 31168, 14)

local function removeTeleport(position)
	local teleportItem = Tile(position):getItemById(1387)
	if teleportItem then
		teleportItem:remove()
		position:sendMagicEffect(CONST_ME_POFF)
	end
end

function onKill(creature, target)
	if not target then
		return true
	end

	if not target or target:getName():lower() ~= 'azerus' then
		return true
	end

	local position = target:getPosition()

	-- Check if teleport already exists at this position
	local teleportItem = Tile(position):getItemById(1387)
	if teleportItem then
		return true
	end

	position:sendMagicEffect(CONST_ME_TELEPORT)
	local item = Game.createItem(1387, 1, position)
	if item:isTeleport() then
		item:setDestination(teleportToPosition)
	end
	target:say('Azerus ran into teleporter! It will disappear in 2 minutes. Enter it!', TALKTYPE_MONSTER_SAY, 0, 0, position)

	-- End the fight immediately
	Game.setStorageValue(GlobalStorage.InServiceOfYalahar.LastFight, 0)

	--remove portal after 2 min
	addEvent(removeTeleport, 2 * 60 * 1000, position)

	--clean arena of monsters
	local spectators, spectator = Game.getSpectators(Position(32783, 31166, 10), false, false, 10, 10, 10, 10)
	for i = 1, #spectators do
		spectator = spectators[i]
		if spectator:isMonster() then
			spectator:getPosition():sendMagicEffect(CONST_ME_POFF)
			spectator:remove()
		end
	end
	return true
end
