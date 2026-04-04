function onStepOut(creature, item, position, fromPosition, toPosition, isMoving)
	local player = creature:getPlayer()
	if not player then
		return true
	end

	-- Check if this is a door that should close
	local doorId = item.itemid
	local isOpen = (doorId % 2) == 0 -- Even IDs are open doors

	if isOpen then
		-- Door is open, close it after player steps off
		addEvent(function()
			local tile = Tile(position)
			if tile then
				local doorItem = tile:getItemById(doorId)
				if doorItem then
					-- Check if no creatures are on the door tile
					local creatures = tile:getCreatures()
					local hasPlayer = false
					for i = 1, #creatures do
						if creatures[i]:isPlayer() then
							hasPlayer = true
							break
						end
					end

					if not hasPlayer then
						doorItem:transform(doorId - 1) -- Close the door
					end
				end
			end
		end, 100) -- Small delay to ensure player has moved off
	end

	return true
end



