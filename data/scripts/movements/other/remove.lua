local items = {
	2128, 2129, 2130
}

local event = MoveEvent()

function event.onStepIn(creature, item, position, fromPosition)
	item:remove(1)
	return true
end

for _, id in pairs(items) do
	event:id(id)
end

event:tileItem(true)
event:register()
