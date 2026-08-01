local moveevent = MoveEvent()

-- 772 SeparationEvent on quest/level open doors (`moveuse.cc:2327-2339`):
-- ClearField(door, exclude=walker) then Change to closed UNPASS.
function moveevent.onStepOut(creature, item, position, fromPosition)
	Game.clearField(item, creature)
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
