local pastries = {
	[2693] = 2689, -- Lump of dough → Bread
	[6277] = 6278, -- Cake dough → Cake
	[9113] = 9111, -- Lump of garlic dough → Garlic bread
	[8846] = 8847  -- → Chocolate cake
}

function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	-- Check if target is an oven (various oven item IDs)
	local ovenIds = {1786, 1787, 1788, 1789, 1790, 1791, 1792, 1793}
	local isOven = false
	for _, ovenId in ipairs(ovenIds) do
		if target.itemid == ovenId then
			isOven = true
			break
		end
	end

	if not isOven then
		return false
	end

	-- Special handling for baking tray with garlic dough
	if item.itemid == 9115 then -- Baking tray with garlic cookie dough
		item:transform(2561) -- Transform back to empty baking tray
		player:addItem(9116, 12) -- Add 12 garlic cookies
		toPosition:sendMagicEffect(CONST_ME_HITBYFIRE)
		return true
	end

	-- Check if the used item is dough that can be baked
	local pastryId = pastries[item.itemid]
	if not pastryId then
		return false
	end

	-- Transform the dough into baked goods
	item:transform(pastryId)
	toPosition:sendMagicEffect(CONST_ME_HITBYFIRE)
	return true
end
