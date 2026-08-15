-- 772 INFORMATION_TIME (`moveuse.cc` UseAnnouncer / objects.srv InformationType=2).
-- Pendulum 1728–1731, watch 2036, cuckoo 1873–1877 and 1881. Sundial is not a 772 clock.

local items = {
	1728, 1729, 1730, 1731,
	2036,
	1873, 1874, 1875, 1876, 1877, 1881,
}

local action = Action()

function action.onUse(player, item, fromPosition, target, toPosition, isHotkey)
	player:sendTextMessage(MESSAGE_INFO_DESCR, "The time is " .. getFormattedWorldTime() .. ".")
	return true
end

for _, id in ipairs(items) do
	action:id(id)
end

action:register()
