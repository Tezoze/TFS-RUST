-- Blood Brothers Quest - Vampire Brothers Kill Script
-- Handles killing of the four vampire brothers: Boreth, Lersatio, Marziel, and Arthei

local vampireBrothers = {
	['boreth'] = {storage = Storage.BloodBrothers.BorekthKill, mission = Storage.BloodBrothers.Mission07},
	['lersatio'] = {storage = Storage.BloodBrothers.LersatioKill, mission = Storage.BloodBrothers.Mission08},
	['marziel'] = {storage = Storage.BloodBrothers.MarzielKill, mission = Storage.BloodBrothers.Mission09},
	['arthei'] = {storage = Storage.BloodBrothers.ArtheiKill, mission = Storage.BloodBrothers.Mission10}
}

function onKill(player, target)
	if not target:isMonster() then
		return true
	end

	local targetMonster = target

	local targetName = targetMonster:getName():lower()
	local bossConfig = vampireBrothers[targetName]
	if not bossConfig then
		return true
	end

	-- Check if player is on the correct mission
	local missionStorage = player:getStorageValue(bossConfig.mission)
	if missionStorage ~= 1 then
		return true
	end

	-- Mark boss as killed
	player:setStorageValue(bossConfig.storage, 1)
	
	-- Special messages for each brother
	if targetName == 'boreth' then
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have destroyed Boreth, the plant-obsessed vampire brother!")
		player:say("The curse of Boreth is broken! The plants around the castle seem to wither.", TALKTYPE_MONSTER_SAY)
	elseif targetName == 'lersatio' then
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have destroyed Lersatio, the vain vampire brother!")
		player:say("Lersatio's vanity has been his downfall! All mirrors in the castle crack.", TALKTYPE_MONSTER_SAY)
	elseif targetName == 'marziel' then
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have destroyed Marziel, the tormented author!")
		player:say("Marziel's suffering is finally over. His diary burns away to ashes.", TALKTYPE_MONSTER_SAY)
		-- Special requirement: Must be female character
		if player:getSex() ~= PLAYERSEX_FEMALE then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Warning: This mission typically requires a female character, but you have completed it anyway.")
		end
	elseif targetName == 'arthei' then
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have destroyed Arthei, the master of the vampire brothers!")
		player:say("With Arthei's destruction, the vampire curse over these lands is finally broken!", TALKTYPE_MONSTER_SAY)
	end

	return true
end
