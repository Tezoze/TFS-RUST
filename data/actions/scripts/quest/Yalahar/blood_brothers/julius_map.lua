-- Julius' Map - Vengoth Spot Marking
-- This script handles marking all Vengoth strange spots with Julius' map

function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	-- Check if player is using Julius' map
	if item.itemid ~= 9117 then
		return false
	end

	-- Check if player has Mission 4 active
	if player:getStorageValue(Storage.BloodBrothers.Mission04) ~= 1 then
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You don't have a mission to mark strange spots.")
		return true
	end

	local playerPos = player:getPosition()

	-- Define all Vengoth spots with their properties
	local spots = {
		{
			name = "the bottomless pit",
			storage = Storage.BloodBrothers.VengothSpots.BottomlessPit,
			center = Position(32949, 31520, 7), -- UPDATE: Pitch Black Gap coordinates
			radius = 3
		},
		{
			name = "the bone circle",
			storage = Storage.BloodBrothers.VengothSpots.BoneCircle,
			center = Position(32942, 31494, 7), -- UPDATE: 6 Bone Totems coordinates
			radius = 3
		},
		{
			name = "the haunted ruin",
			storage = Storage.BloodBrothers.VengothSpots.HauntedRuin,
			center = Position(32914, 31490, 7), -- UPDATE: Building with A Wandering Soul coordinates
			radius = 3
		},
		{
			name = "the lonely grave",
			storage = Storage.BloodBrothers.VengothSpots.LonelyGrave,
			center = Position(32904, 31470, 7), -- UPDATE: Grave with inscription coordinates
			radius = 3
		},
		{
			name = "the miraculously burning trees",
			storage = Storage.BloodBrothers.VengothSpots.BurningTrees,
			center = Position(32882, 31500, 7), -- UPDATE: Dead Trees burning coordinates
			radius = 3
		},
		{
			name = "the old shrine",
			storage = Storage.BloodBrothers.VengothSpots.OldShrine,
			center = Position(32934, 31562, 4), -- UPDATE: Mountain shrine coordinates
			radius = 3
		},
		{
			name = "the castle garden",
			storage = Storage.BloodBrothers.VengothSpots.CastleGarden,
			center = Position(32963, 31496, 6), -- UPDATE: Castle garden coordinates
			radius = 3
		},
		{
			name = "the castle entrance",
			storage = Storage.BloodBrothers.VengothSpots.CastleEntrance,
			center = Position(32953, 31487, 6), -- UPDATE: Castle entrance coordinates
			radius = 3
		}
	}

	-- Check if player is near any spot
	for _, spot in ipairs(spots) do
		local distance = math.max(math.abs(playerPos.x - spot.center.x), math.abs(playerPos.y - spot.center.y))

		if distance <= spot.radius and playerPos.z == spot.center.z then
			-- Player is at this spot, check if already marked
			if player:getStorageValue(spot.storage) == 1 then
				player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have already marked this spot on your map.")
				return true
			end

			-- Mark the spot
			player:setStorageValue(spot.storage, 1)
			-- Increment total marked count
			local currentCount = player:getStorageValue(Storage.BloodBrothers.VengothSpots.MarkedCount) or 0
			player:setStorageValue(Storage.BloodBrothers.VengothSpots.MarkedCount, currentCount + 1)
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have marked " .. spot.name .. " on Julius' map.")
			playerPos:sendMagicEffect(CONST_ME_MAGIC_BLUE)
			return true
		end
	end

	-- Player is not near any spot
	player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "There is nothing unusual here to mark on your map.")
	return true
end
