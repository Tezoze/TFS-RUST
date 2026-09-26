local action = Action()

-- 772 Fun MultiUse 2977+2917; E2: zero-thing is a table, not Item userdata.
function action.onUse(player, item, fromPosition, target, toPosition, isHotkey)
	if type(target) ~= "userdata" or not target:isItem() then
		return false
	end
	
	if target:getId() == 2917 then
		item:transform(2978, 1)
		item:decay()
		target:remove()
		return true
	end
	return false
end

action:id(2977)
action:register()
