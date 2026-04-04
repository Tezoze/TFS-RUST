local waterIds = {493, 4608, 4609, 4610, 4611, 4612, 4613, 4614, 4615, 4616, 4617, 4618, 4619, 4620, 4621, 4622, 4623, 4624, 4625, 7236, 10499, 15401, 15402}
local useWorms = true

-- Water Elemental loot table with proper chances from water_elemental.xml
local waterElementalLoot = {
	{itemId = 2145, chance = 1000, count = 1},      -- small diamond
	{itemId = 2146, chance = 1000, count = 1},      -- small sapphire
	{itemId = 2148, chance = 50000, countMax = 100}, -- gold coin
	{itemId = 2149, chance = 1000, countMax = 2},   -- small emerald
	{itemId = 2152, chance = 10000, count = 1},     -- platinum coin
	{itemId = 2167, chance = 950, count = 1},       -- energy ring
	{itemId = 2168, chance = 930, count = 1},       -- life ring
	{itemId = 2667, chance = 20000, count = 1},     -- fish
	{itemId = 7158, chance = 940, count = 1},       -- rainbow trout
	{itemId = 7159, chance = 1050, count = 1},      -- green perch
	{itemId = 7588, chance = 10000, count = 1},     -- strong health potion
	{itemId = 7589, chance = 10000, count = 1},     -- strong mana potion
	{itemId = 7632, chance = 800, count = 1},       -- giant shimmering pearl
	{itemId = 7633, chance = 800, count = 1}        -- giant shimmering pearl
}

function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	local targetId = target.itemid
	if not table.contains(waterIds, targetId) then
		return false
	end

	if targetId == 10499 then
		local owner = target:getAttribute(ITEM_ATTRIBUTE_CORPSEOWNER)
		if owner ~= 0 and owner ~= player:getId() then
			player:sendTextMessage(MESSAGE_STATUS_SMALL, "You are not the owner.")
			return true
		end

		toPosition:sendMagicEffect(CONST_ME_WATERSPLASH)
		target:transform(targetId + 1)
		target:decay()

		-- Generate loot using water elemental's proper loot table
		local lootRate = configManager.getNumber(configKeys.RATE_LOOT)
		for _, lootItem in pairs(waterElementalLoot) do
			-- Apply loot rate multiplier to the chance
			local adjustedChance = lootItem.chance * lootRate
			if math.random(100000) <= adjustedChance then
				local count = lootItem.count or 1
				if lootItem.countMax then
					count = math.random(1, lootItem.countMax)
				end
				player:addItem(lootItem.itemId, count)
			end
		end
		return true
	end

	if targetId ~= 7236 then
		toPosition:sendMagicEffect(CONST_ME_LOSEENERGY)
	end

	if targetId == 493 or targetId == 15402 then
		return true
	end

	player:addSkillTries(SKILL_FISHING, 1)
	if math.random(1, 100) <= math.min(math.max(10 + (player:getEffectiveSkillLevel(SKILL_FISHING) - 10) * 0.597, 10), 50) then
		if useWorms and not player:removeItem(3976, 1) then
			return true
		end

		if targetId == 15401 then
			target:transform(targetId + 1)
			target:decay()

			if math.random(1, 100) >= 97 then
				player:addItem(15405, 1)
				player:addAchievement("Desert Fisher")
				return true
			end
		elseif targetId == 7236 then
			target:transform(targetId + 1)
			target:decay()
			player:addAchievementProgress("Exquisite Taste", 250)

			local rareChance = math.random(1, 100)
			if rareChance == 1 then
				player:addItem(7158, 1)
				return true
			elseif rareChance <= 4 then
				player:addItem(2669, 1)
				return true
			elseif rareChance <= 10 then
				player:addItem(7159, 1)
				return true
			end
		end
		player:addAchievementProgress("Here, Fishy Fishy!", 1000)
		player:addItem(2667, 1)
	end
	return true
end
