-- 772 INFORMATION_TIME (`moveuse.cc` UseAnnouncer / objects.srv InformationType=2).
-- Pendulum 1728–1731, watch 2036, cuckoo 1873–1877 and 1881. Sundial is not a 772 clock.

local items = {
	2445, 2446, 2447, 2448,
	2906,
	2660, 2661, 2662, 2663, 2664, 2668,
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
