local moveevent = MoveEvent()

function moveevent.onStepOut(creature, item, position, fromPosition)
	local tile = Tile(position)
	if tile:getCreatureCount() > 0 then
		return true
	end

	local newPosition = {x = position.x + 1, y = position.y, z = position.z}
	local query = Tile(newPosition):queryAdd(creature)
	if query ~= RETURNVALUE_NOERROR or query == RETURNVALUE_NOTENOUGHROOM then
		newPosition.x = newPosition.x - 1
		newPosition.y = newPosition.y + 1
		query = Tile(newPosition):queryAdd(creature)
	end

	if query == RETURNVALUE_NOERROR or query ~= RETURNVALUE_NOTENOUGHROOM then
		doRelocate(position, newPosition)
	end

	local i, tileItem, tileCount = 1, true, tile:getThingCount()
	while tileItem and i < tileCount do
		tileItem = tile:getThing(i)
		if tileItem and tileItem:getUniqueId() ~= item.uid and tileItem:getType():isMovable() then
			newPosition = {x = position.x, y = position.y - 1, z = position.z}
			tileItem:moveTo(newPosition, 0)
		end

		i = i + 1
	end
	
	local splashItem = tile:getItemByGroup(ITEM_GROUP_SPLASH)
	if splashItem then
		splashItem:remove()
	end
	
	local magicField = tile:getItemByGroup(ITEM_GROUP_MAGICFIELD)
	if magicField then
		magicField:remove()
	end

	item:transform(item.itemid - 1)
	return true
end

for _, id in pairs(openLevelDoors) do
	moveevent:id(id)
end
for _, id in pairs(openQuestDoors) do
	moveevent:id(id)
end

moveevent:register()