-- 772 Instruments (`moveuse.dat:129-152`) + Fun cornucopia TypeID 3103 (`:38-39`).
-- `Random(n)` is `math.random(1,100) <= n` (`moveuse.cc:349-350`).
-- Bongo 3951 / war drum 3953 have no 772 UseEvent; gated on extraInstruments.

local instruments = {
	[2948] = {effect = CONST_ME_SOUND_GREEN}, -- wooden flute
	[2949] = {effect = CONST_ME_SOUND_GREEN}, -- lyre
	[2950] = {effect = CONST_ME_SOUND_GREEN}, -- lute
	[2952] = {effect = CONST_ME_SOUND_GREEN}, -- drum
	[2953] = {effect = CONST_ME_SOUND_GREEN}, -- panpipes
	[2954] = {effect = CONST_ME_SOUND_GREEN}, -- simple fanfare
	[2955] = {effect = CONST_ME_SOUND_GREEN}, -- fanfare
	[2956] = {effect = CONST_ME_SOUND_GREEN}, -- royal fanfare
	[2957] = {effect = CONST_ME_SOUND_GREEN}, -- post horn
	[2958] = {effect = CONST_ME_SOUND_GREEN}, -- war horn
	[2959] = {effects = {failure = CONST_ME_SOUND_PURPLE, success = CONST_ME_SOUND_GREEN}, chance = 50}, -- piano
	[2960] = {effects = {failure = CONST_ME_SOUND_PURPLE, success = CONST_ME_SOUND_GREEN}, chance = 50}, -- piano
	[2961] = {effects = {failure = CONST_ME_SOUND_PURPLE, success = CONST_ME_SOUND_GREEN}, chance = 50}, -- piano
	[2962] = {effects = {failure = CONST_ME_SOUND_PURPLE, success = CONST_ME_SOUND_GREEN}, chance = 50}, -- piano
	[2963] = {effect = CONST_ME_SOUND_GREEN}, -- harp
	[2964] = {effect = CONST_ME_SOUND_GREEN}, -- harp
	[3219] = {effect = CONST_ME_SOUND_GREEN}, -- Waldo's post horn
	[3255] = {effect = CONST_ME_SOUND_GREEN}, -- drum (immovable)
	[3256] = {effect = CONST_ME_SOUND_GREEN}, -- simple fanfare (immovable)
	[3257] = {effect = CONST_ME_SOUND_GREEN}, -- horn (immovable); not cornucopia
	[3258] = {effect = CONST_ME_SOUND_GREEN}, -- lute (immovable)
	[3259] = {effect = CONST_ME_SOUND_BLUE}, -- horn
	[3260] = {effect = CONST_ME_SOUND_GREEN}, -- lyre (immovable)
	[3261] = {effect = CONST_ME_SOUND_GREEN}, -- panpipes (immovable)
	[2965] = {effects = {failure = CONST_ME_POFF, success = CONST_ME_SOUND_GREEN}, chance = 10}, -- didgeridoo TypeID 2965
	-- Fun 3103: Random(95) Effect 19 + Create×10 grapes; else Effect 19 + Create×9 + Change→grapes.
	[3103] = {effect = CONST_ME_SOUND_GREEN, itemId = 3592, itemCount = 10, failItemCount = 9, chance = 95, transformOnFail = 3592},
}

if formulas and formulas.otherActions and formulas.otherActions.extraInstruments then
	instruments[2951] = {effect = CONST_ME_SOUND_BLUE} -- bongo drum
	instruments[2966] = {effect = CONST_ME_SOUND_RED} -- war drum
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
