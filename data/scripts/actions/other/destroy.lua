local items = {
	{fromid=3264, toid=3292},
	{fromid=3294, toid=3303},
	{fromid=3305, toid=3307},
	{fromid=3309, toid=3329},
	{fromid=3331, toid=3341},
}

local action = Action()

function action.onUse(player, item, fromPosition, target, toPosition)
	return destroyItem(player, target, toPosition)
end

for _, data in ipairs(items) do
	for i=data.fromid, data.toid, 1 do
		action:id(i)
	end
end

action:register()
