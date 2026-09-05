-- 772 Fun TypeID 2976 (`moveuse.dat:36-37`): empty iff Random(1) and Random(10)
-- (0.1%); else Effect 22. Change 2976→2975 = OTB 2095→2094.
-- `Random(n)` is `math.random(1,100) <= n` (`moveuse.cc:349-350`).

local action = Action()

function action.onUse(player, item, fromPosition, target, toPosition)
	if math.random(1, 100) <= 1 and math.random(1, 100) <= 10 then
		item:transform(2094)
	else
		item:getPosition():sendMagicEffect(CONST_ME_SOUND_YELLOW)
	end
	return true
end

action:id(2095)
action:register()
