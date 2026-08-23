-- Pack policy for player item moves. Native cylinder move runs first (queryAdd);
-- this callback may cancel (quest aid / blocking tile) or mutate (candelabrum).
-- onItemMoved runs after a successful transfer (open trap).
--
-- Rehomed from data/events/scripts/player.lua (Phase 5). Registration is the
-- enable bit — this file is on the scripts-interface allowlist.

local ec = EventCallback

ec.onMoveItem = function(player, item, count, fromPosition, toPosition, fromCylinder, toCylinder)
	-- Do not allow moving of map quest objects
	if item:getActionId() >= 1000 and item:getActionId() <= 2000 then
		return RETURNVALUE_NOTMOVEABLE
	end

	-- Convert permanent candelabrum into expiring candelabrum (pre-move)
	if item:getId() == 2057 then
		item:transform(2042)
	end

	if toCylinder:isTile() then
		local ground = toCylinder:getGround()
		if ground and actionIds and ground:getActionId() == actionIds.blockingTile then
			return RETURNVALUE_NOTENOUGHROOM
		end
	end

	return RETURNVALUE_NOERROR
end
ec:register()

ec.onItemMoved = function(player, item, count, fromPosition, toPosition, fromCylinder, toCylinder)
	-- Open trap
	if item:getId() == 2579 then
		item:transform(2578)
		toCylinder:getPosition():sendMagicEffect(CONST_ME_POFF)
	end
end
ec:register()
