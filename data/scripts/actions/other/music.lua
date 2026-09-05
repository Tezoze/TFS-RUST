-- 772 Instruments (`moveuse.dat:129-152`) + Fun cornucopia TypeID 3103 (`:38-39`).
-- `Random(n)` is `math.random(1,100) <= n` (`moveuse.cc:349-350`).
-- Bongo 3951 / war drum 3953 have no 772 UseEvent; gated on extraInstruments.

local instruments = {
	[2070] = {effect = CONST_ME_SOUND_GREEN}, -- wooden flute
	[2071] = {effect = CONST_ME_SOUND_GREEN}, -- lyre
	[2072] = {effect = CONST_ME_SOUND_GREEN}, -- lute
	[2073] = {effect = CONST_ME_SOUND_GREEN}, -- drum
	[2074] = {effect = CONST_ME_SOUND_GREEN}, -- panpipes
	[2075] = {effect = CONST_ME_SOUND_GREEN}, -- simple fanfare
	[2076] = {effect = CONST_ME_SOUND_GREEN}, -- fanfare
	[2077] = {effect = CONST_ME_SOUND_GREEN}, -- royal fanfare
	[2078] = {effect = CONST_ME_SOUND_GREEN}, -- post horn
	[2079] = {effect = CONST_ME_SOUND_GREEN}, -- war horn
	[2080] = {effects = {failure = CONST_ME_SOUND_PURPLE, success = CONST_ME_SOUND_GREEN}, chance = 50}, -- piano
	[2081] = {effects = {failure = CONST_ME_SOUND_PURPLE, success = CONST_ME_SOUND_GREEN}, chance = 50}, -- piano
	[2082] = {effects = {failure = CONST_ME_SOUND_PURPLE, success = CONST_ME_SOUND_GREEN}, chance = 50}, -- piano
	[2083] = {effects = {failure = CONST_ME_SOUND_PURPLE, success = CONST_ME_SOUND_GREEN}, chance = 50}, -- piano
	[2084] = {effect = CONST_ME_SOUND_GREEN}, -- harp
	[2085] = {effect = CONST_ME_SOUND_GREEN}, -- harp
	[2332] = {effect = CONST_ME_SOUND_GREEN}, -- Waldo's post horn
	[2367] = {effect = CONST_ME_SOUND_GREEN}, -- drum (immovable)
	[2368] = {effect = CONST_ME_SOUND_GREEN}, -- simple fanfare (immovable)
	[2369] = {effect = CONST_ME_SOUND_GREEN}, -- horn (immovable); not cornucopia
	[2370] = {effect = CONST_ME_SOUND_GREEN}, -- lute (immovable)
	[2371] = {effect = CONST_ME_SOUND_BLUE}, -- horn
	[2372] = {effect = CONST_ME_SOUND_GREEN}, -- lyre (immovable)
	[2373] = {effect = CONST_ME_SOUND_GREEN}, -- panpipes (immovable)
	[3952] = {effects = {failure = CONST_ME_POFF, success = CONST_ME_SOUND_GREEN}, chance = 10}, -- didgeridoo TypeID 2965
	-- Fun 3103: Random(95) Effect 19 + Create×10 grapes; else Effect 19 + Create×9 + Change→grapes.
	[3957] = {effect = CONST_ME_SOUND_GREEN, itemId = 2681, itemCount = 10, failItemCount = 9, chance = 95, transformOnFail = 2681},
}

if formulas and formulas.otherActions and formulas.otherActions.extraInstruments then
	instruments[3951] = {effect = CONST_ME_SOUND_BLUE} -- bongo drum
	instruments[3953] = {effect = CONST_ME_SOUND_RED} -- war drum
end

local action = Action()

function action.onUse(player, item, fromPosition, target, toPosition)
	local instrument, chance = instruments[item:getId()]
	if instrument.chance then
		chance = instrument.chance >= math.random(1, 100)

		if instrument.monster and chance then
			local monster = Game.createMonster(instrument.monster, player:getPosition(), true)
			if monster then
				player:addSummon(monster)
			end
		elseif instrument.itemId then
			local count = chance and instrument.itemCount or instrument.failItemCount
			if count then
				player:addItem(instrument.itemId, count)
			end
		end
	end

	item:getPosition():sendMagicEffect(instrument.effect or instrument.effects and chance and instrument.effects.success or instrument.effects.failure)

	if instrument.transformId then
		player:say(instrument.text, TALKTYPE_MONSTER_SAY, false, nil, item:getPosition())
		item:transform(instrument.transformId)
		item:decay(instrument.decayId)
	end

	if not chance and instrument.transformOnFail then
		item:transform(instrument.transformOnFail)
	end
	return true
end

for id in pairs(instruments) do
	action:id(id)
end

action:register()
