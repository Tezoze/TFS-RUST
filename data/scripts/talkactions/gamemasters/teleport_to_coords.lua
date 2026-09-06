local talkaction = TalkAction("/coord")

function talkaction.onSay(player, words, param)
	if not player:getGroup():getAccess() then
		return true
	end

	local x, y, z = param:match("(%d+)%D+(%d+)%D+(%d+)")
	x, y, z = tonumber(x), tonumber(y), tonumber(z)
	if not x or not y or not z then
		player:sendCancelMessage("Usage: /coord x y z")
		return false
	end

	player:teleportTo(Position(x, y, z))
	return false
end

talkaction:separator(" ")
talkaction:register()
