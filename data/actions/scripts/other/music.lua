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
	[2369] = {effect = CONST_ME_SOUND_YELLOW, itemId = 2681, itemCount = 10, chance = 80, remove = true}, -- cornucopia (immovable)
	[2370] = {effect = CONST_ME_SOUND_GREEN}, -- lute (immovable)
	[2371] = {effect = CONST_ME_SOUND_BLUE}, -- the horn of sundering (actual effect is unknown; immovable)
	[2372] = {effect = CONST_ME_SOUND_GREEN}, -- lyre (immovable)
	[2373] = {effect = CONST_ME_SOUND_GREEN}, -- panpipes (immovable)
	[3951] = {effect = CONST_ME_SOUND_BLUE}, -- bongo drum (actual effect is unknown)
	[3952] = {effects = {failure = CONST_ME_POFF, success = CONST_ME_SOUND_GREEN}, chance = 20}, -- didgeridoo
	[3953] = {effect = CONST_ME_SOUND_RED}, -- war drum
	[3957] = {effect = CONST_ME_SOUND_YELLOW, itemId = 2681, itemCount = 10, chance = 80, remove = true}, -- cornucopia
	[5786] = {effects = {failure = CONST_ME_SOUND_RED, success = CONST_ME_SOUND_YELLOW}, monster = "war wolf", chance = 60, remove = true}, -- wooden whistle
	[6572] = {effect = CONST_ME_SOUND_GREEN, text = "TOOOOOOT", transformId = 13578, decayId = 6572}, -- party trumpet
	[6573] = {effect = CONST_ME_SOUND_GREEN, text = "TOOOOOOT", transformId = 13578, decayId = 6573}, -- party trumpet
	[13759] = {effect = CONST_ME_SOUND_BLUE}, -- small whistle (actual effect is unknown)
	[23923] = {effect = CONST_ME_SOUND_WHITE} -- small crystal bell
}

function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	-- Special handling for An Uneasy Alliance Foul Spirits task
	if item:getId() == 2070 and player:getStorageValue(Storage.AnUneasyAllianceTasks.FoulSpirits) == 1 then
		-- Haunted trees location in Zao - set these coordinates in your map editor
		local hauntedTreesPos = Position(33298, 31387, 7) 

		if player:getPosition() == hauntedTreesPos then
			-- 50% chance to fail and spawn monsters
			if math.random(1, 100) <= 50 then
				-- Failure - spawn monsters
				local monsters = {"Haunted Treeling", "Carniphila"}
				local chosenMonster = monsters[math.random(#monsters)]

				-- Spawn 2 monsters around the player
				for i = 1, 2 do
					local offsetX = math.random(-2, 2)
					local offsetY = math.random(-2, 2)
					local spawnPos = Position(hauntedTreesPos.x + offsetX, hauntedTreesPos.y + offsetY, hauntedTreesPos.z)

					if spawnPos ~= hauntedTreesPos then -- Don't spawn on the flute playing position
						local monster = Game.createMonster(chosenMonster, spawnPos, true)
						if monster then
							monster:setTarget(player)
						end
					end
				end

				-- 30% chance to get cursed
				if math.random(1, 100) <= 30 then
					player:addCondition(Condition(CONDITION_CURSED, 20, 15)) -- 20 turns, 15 damage per turn = 300 total
					player:say("You have been cursed by the angry spirit!", TALKTYPE_MONSTER_SAY)
				end

				player:say("The tree spirit becomes enraged and summons creatures to attack you!", TALKTYPE_MONSTER_SAY)
				fromPosition:sendMagicEffect(CONST_ME_SOUND_RED)
				player:addAchievementProgress("Rockstar", 10000)
				return true
			else
				-- Success - complete the task
				player:setStorageValue(Storage.AnUneasyAllianceTasks.FoulSpirits, 2)
				player:say("You play the flute and the tree spirit calms down, its evil presence fading away.", TALKTYPE_MONSTER_SAY)
				fromPosition:sendMagicEffect(CONST_ME_SOUND_BLUE)
				player:addAchievementProgress("Rockstar", 10000)
				return true
			end
		else
			player:say("This doesn't seem like the right place to play the flute for the tree spirit.", TALKTYPE_MONSTER_SAY)
			fromPosition:sendMagicEffect(CONST_ME_SOUND_GREEN)
			player:addAchievementProgress("Rockstar", 10000)
			return true
		end
	end

	local instrument, chance = instruments[item:getId()]
	if instrument.chance then
		chance = instrument.chance >= math.random(1, 100)

		if instrument.monster and chance then
			local monster = Game.createMonster(instrument.monster, player:getPosition(), true)
			if monster then
				player:addSummon(monster)
			end
		elseif instrument.itemId and chance then
			player:addItem(instrument.itemId, instrument.itemCount)
		end
	end

	item:getPosition():sendMagicEffect(instrument.effect or instrument.effects and chance and instrument.effects.success or instrument.effects.failure)

	if instrument.transformId then
		player:say(instrument.text, TALKTYPE_MONSTER_SAY, false, nil, item:getPosition())
		item:transform(instrument.transformId)
		item:decay(instrument.decayId)
	end

	if not chance and instrument.remove then
		item:remove()
	end
	player:addAchievementProgress("Rockstar", 10000)
	return true
end
