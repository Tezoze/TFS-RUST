function onAddItem(moveitem, tileitem, position)
	-- Check if ItemTeleports table exists
	if not ItemTeleports then
		return true
	end
	
	local setting = ItemTeleports[tileitem.actionid]
	if not setting then
		return true
	end	
	
	moveitem:moveTo(setting.destination)
	setting.destination:sendMagicEffect(setting.effect)
	return true
end
