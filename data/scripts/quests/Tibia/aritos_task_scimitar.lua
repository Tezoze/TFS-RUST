-- Arito's Task - Scimitar Movement Event
-- When a scimitar is dropped/moved to position {33205, 32537, 6}, check for cave entrance

local scimitarPos = Position(33205, 32537, 6)
local waterPos = Position(33207, 32537, 6)
local caveEntrancePos = Position(33206, 32536, 6)
local scimitarItemId = 2419  -- scimitar
local placedScimitarItemId = 5858  -- placed scimitar decoration
local caveEntranceId = 8210  -- cave entrance (gate)
local wallItemId = 877  -- wall/rock that transforms

local scimitarMoveEvent = MoveEvent()

function scimitarMoveEvent.onAddItem(moveitem, tileitem, position)
	-- Only trigger at the specific scimitar position
	if position ~= scimitarPos then
		return true
	end
	
	-- Check if it's a scimitar being placed
	if moveitem:getId() ~= scimitarItemId then
		return true
	end
	
	-- Call the checkWallArito function with the scimitar
	checkWallArito(moveitem, position)
	
	return true
end

scimitarMoveEvent:position(scimitarPos)
scimitarMoveEvent:register()
