function onCastSpell(creature, variant)
	-- conjureItem doesn't exist in TFS 1.4, implement manually
	-- Parameters: reagentId (2260 = blank rune), productId (2290 = convince creature rune), count
	local player = creature:getPlayer()
	if not player then
		return false
	end

	local reagentId = 2260  -- blank rune
	local productId = 2290  -- convince creature rune
	local count = 1

	-- Check if player has the reagent
	local reagentCount = player:getItemCount(reagentId)
	if reagentCount < 1 then
		player:sendCancelMessage("You need a blank rune.")
		player:getPosition():sendMagicEffect(CONST_ME_POFF)
		return false
	end

	-- Remove reagent and add product
	player:removeItem(reagentId, 1)
	player:addItem(productId, count)
	player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
	return true
end
