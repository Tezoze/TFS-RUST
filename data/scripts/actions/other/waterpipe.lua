-- 772 Fun TypeID 2974 (`moveuse.dat:34-35`): Random(90) puff on item, else player.
-- Effect 3. OTB 2093 only. 2099 is a TFS extra (`formulas.otherActions.extraInstruments`).

local action = Action()

function action.onUse(player, item, fromPosition, target, toPosition)
	if math.random(1, 100) <= 90 then
		item:getPosition():sendMagicEffect(CONST_ME_POFF)
	else
		player:getPosition():sendMagicEffect(CONST_ME_POFF)
	end
	return true
end

action:id(2093)
if formulas and formulas.otherActions and formulas.otherActions.extraInstruments then
	action:id(2099)
end
action:register()
