local action = Action()

-- 772 Fun MultiUse 2916+2874; E2: zero-thing is a table, not Item userdata.
function action.onUse(player, item, fromPosition, target, toPosition, isHotkey)
	if type(target) ~= "userdata" or not target:isItem() then
		return false
	end
	
	if target:getId() == 2006 and target:getFluidType() == FLUID_OIL then 
		target:transform(target:getId(), FLUID_NONE)
		item:transform(2044, 1) -- brand new lamp
		item:decay()
		return true
	end
	return false
end

action:id(2046) -- used lamp
action:register()
