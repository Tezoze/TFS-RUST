-- 772 `UseAnnouncer` case 3 (`moveuse.cc:1909-1944`) — ceremonial mask TypeID 3395 / OTB 2501.
-- Quest values 101–105; none → "No blessings received."

local BLESSINGS = {
	{ storage = 101, name = "Wisdom of Solitude" },
	{ storage = 102, name = "Spark of the Phoenix" },
	{ storage = 103, name = "Fire of the Suns" },
	{ storage = 104, name = "Spiritual Shielding" },
	{ storage = 105, name = "Embrace of Tibia" },
}

local action = Action()

function action.onUse(player, item, fromPosition, target, toPosition, isHotkey)
	local names = {}
	for _, b in ipairs(BLESSINGS) do
		if player:getStorageValue(b.storage) ~= -1 and player:getStorageValue(b.storage) ~= 0 then
			names[#names + 1] = b.name
		end
	end
	if #names == 0 then
		player:sendTextMessage(MESSAGE_INFO_DESCR, "No blessings received.")
	else
		player:sendTextMessage(MESSAGE_INFO_DESCR, "Received blessings:\n" .. table.concat(names, "\n"))
	end
	return true
end

action:id(2501)
action:register()
