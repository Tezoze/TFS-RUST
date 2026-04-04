-- Julius - Converted from XML to Lua NpcType
-- Original XML: data/npc/Julius.xml
-- Original Script: data/npc/scripts/Julius.lua

local npcName = "Julius"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a julius")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 289, lookHead = 114, lookBody = 114, lookLegs = 114, lookFeet = 113, lookAddons = 3})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	local playername = player:getName()
	
	-- Check if player has completed Inquisition Quest Mission 3: Vampire Hunt
	local hasInquisitionProof = player:getStorageValue(Storage.TheInquisition.Mission03) >= 6
	
	-- Mission start
	if msgcontains(msg, "mission") then
		if player:getStorageValue(Storage.BloodBrothers.Questline) == -1 then
			if hasInquisitionProof then
				-- Skip Mission 1 if completed Inquisition
				player:setStorageValue(Storage.BloodBrothers.Questline, 2)
				player:setStorageValue(Storage.BloodBrothers.Mission01, 2) -- Mark as completed for quest log
				player:setStorageValue(Storage.BloodBrothers.Mission02, 1) -- Mark as active for quest log
				player:setStorageValue(Storage.BloodBrothers.GarlicCookieCount, 0) -- Initialize cookie count for quest log
				npcHandler:say("I see you have proven yourself worthy against vampires before. Let me explain what we need to do...", cid)
				npcHandler:say("Yalahar is under attack by vampires. I need you to help me find vampire suspects among the citizens.", cid)
			else
				-- Start Mission 1 - Requires Garlic Necklace first
				if player:getItemCount(2199) > 0 then -- Garlic Necklace
					player:setStorageValue(Storage.BloodBrothers.Questline, 1)
					player:setStorageValue(Storage.BloodBrothers.Mission01, 1) -- Mark as active for quest log
					npcHandler:say("I see you wear a Garlic Necklace, but that is not enough proof that you are against the vampires. I want you to make Garlic Bread and eat it in front of me.", cid)
					npcHandler:say("Mix flour with holy water, then use that dough on bulb of garlic to create garlic dough. Bake it and eat it here!", cid)
				else
					npcHandler:say("Yalahar is under attack by vampires, " .. playername .. "! But first I need proof that you are not one of them. Come back when you wear a Garlic Necklace.", cid)
				end
			end

		elseif player:getStorageValue(Storage.BloodBrothers.Questline) == 1 then
			-- Mission 1 in progress
			npcHandler:say("Have you prepared the garlic bread? You need to eat it here to prove you're not a vampire!", cid)

		elseif player:getStorageValue(Storage.BloodBrothers.Questline) == 2 then
			-- Mission 1 Complete - Ask for Mission 2
			npcHandler:say("Now that I can trust you, are you ready to help identify vampire suspects in Yalahar?", cid)
			npcHandler.topic[cid] = 26

		elseif player:getStorageValue(Storage.BloodBrothers.Questline) == 3 then
			-- Mission 2 in progress - check if completed
			if player:getStorageValue(Storage.BloodBrothers.GarlicCookieCount) >= 5 then
				npcHandler:say("Have you identified all the vampire suspects among Yalahar's citizens?", cid)
				npcHandler.topic[cid] = 21
			else
				npcHandler:say("I need you to find vampire suspects by giving garlic cookies to citizens of Yalahar. Bake garlic cookies and offer them to Serafin, Lisander, Ortheus, Maris, and Armenius.", cid)
				npcHandler:say("Watch their reactions carefully and remember who refuses or reacts suspiciously!", cid)
			end

		elseif player:getStorageValue(Storage.BloodBrothers.Questline) == 4 then
			-- Mission 2 Complete - Ask for Mission 3
			npcHandler:say("Excellent work identifying the suspects. Are you ready to reveal their true nature?", cid)
			npcHandler.topic[cid] = 27

		elseif player:getStorageValue(Storage.BloodBrothers.Questline) == 5 then
			-- Mission 3 in progress - check if completed
			if player:getStorageValue(Storage.BloodBrothers.Mission03) == 2 then
				npcHandler:say("Have you cast 'alori mort' on Armenius to reveal his true nature?", cid)
				npcHandler.topic[cid] = 22
			else
				npcHandler:say("Cast 'alori mort' on Armenius to reveal his true vampire nature!", cid)
			end

		elseif player:getStorageValue(Storage.BloodBrothers.Questline) == 6 then
			-- Mission 3 Complete - Ask for Mission 4
			npcHandler:say("So the vampire threat is real! Are you ready to journey to Vengoth?", cid)
			npcHandler.topic[cid] = 28

		elseif player:getStorageValue(Storage.BloodBrothers.Questline) == 7 then
			-- Mission 4 in progress - check if completed
			if player:getStorageValue(Storage.BloodBrothers.Mission04) == 2 then
				npcHandler:say("Have you found a way to enter the vampire castle in Vengoth?", cid)
				npcHandler.topic[cid] = 23
			else
				npcHandler:say("Have you been to Vengoth yet? Find Harlow at the docks for passage there, then explore and try to enter the vampire castle!", cid)
			end

		elseif player:getStorageValue(Storage.BloodBrothers.Questline) == 8 then
			-- Mission 4 Complete - Ask for Mission 5
			npcHandler:say("The ghostly guardians are a problem. Are you ready to find a way past them?", cid)
			npcHandler.topic[cid] = 29
		
		elseif player:getStorageValue(Storage.BloodBrothers.Questline) == 9 then
			-- Mission 5 - Blood Crystal Ritual
			local hasUnchargedCrystal = player:getItemCount(9369) > 0 -- Uncharged Blood Crystal
			local hasChargedCrystal = player:getItemCount(9141) > 0 -- Charged Blood Crystal
			local crystalCharged = player:getStorageValue(Storage.BloodBrothers.BloodCrystal.Charged) == 1
			local ritualCompleted = player:getStorageValue(Storage.BloodBrothers.BloodCrystal.RitualCompleted) == 1

			if not hasUnchargedCrystal and not hasChargedCrystal and not crystalCharged and not ritualCompleted then
				-- Start of mission - needs blood crystal
				npcHandler:say("To get deeper into the castle, we need a special ritual. First, get a Blood Crystal from the Research Centre in the Magician Quarter.", cid)
				npcHandler:say("Ask around Yalahar about 'blood crystal' for hints on where to find it.", cid)
				player:setStorageValue(Storage.BloodBrothers.BloodCrystal.Quest, 1)
			elseif hasUnchargedCrystal and not crystalCharged then
				-- Has uncharged crystal, start dialogue about charging
				npcHandler:say("As I said, I don't know where you might get a blood crystal - but did you find one?", cid)
				npcHandler.topic[cid] = 10
			elseif (hasChargedCrystal or crystalCharged) and not ritualCompleted then
				-- Crystal charged, give ritual mission
				player:setStorageValue(Storage.BloodBrothers.Questline, 10)
				player:setStorageValue(Storage.BloodBrothers.Mission05, 3) -- Update quest log
				npcHandler:say("Your crystal is charged! Now gather 3 other players (total 4) with charged blood crystals.", cid)
				npcHandler:say("Each of you must stand on one of the 4 strange carvings in Vengoth. When the 4th person steps on their carving, you'll all be teleported deeper into the castle.", cid)
			elseif ritualCompleted then
				npcHandler:say("Have you completed the blood crystal ritual with the other players?", cid)
				npcHandler.topic[cid] = 24
			else
				npcHandler:say("What's your progress with the Blood Crystal ritual?", cid)
			end

		elseif player:getStorageValue(Storage.BloodBrothers.Questline) == 10 then
			-- Mission 5 - Ritual in progress
			local ritualCompleted = player:getStorageValue(Storage.BloodBrothers.BloodCrystal.RitualCompleted) == 1
			if ritualCompleted then
				npcHandler:say("Have you completed the blood crystal ritual?", cid)
				npcHandler.topic[cid] = 24
			else
				npcHandler:say("You haven't completed the ritual yet. Gather your charged blood crystal and 3 other players to perform the ritual on the strange carvings.", cid)
			end

		elseif player:getStorageValue(Storage.BloodBrothers.Questline) == 11 then
			-- Mission 5 Complete - Ask for Mission 6
			npcHandler:say("You've made it deeper into the castle. Are you ready to explore further?", cid)
			npcHandler.topic[cid] = 30

		elseif player:getStorageValue(Storage.BloodBrothers.Questline) == 12 then
			-- Mission 6 - A Black History
			npcHandler:say("Ah! Welcome back! So you have been inside the castle? Was it as spooky as in the stories told by people?", cid)
			npcHandler.topic[cid] = 13

		elseif player:getStorageValue(Storage.BloodBrothers.Questline) == 13 then
			-- Mission 6 - Return with diary - check if completed
			if player:getStorageValue(Storage.BloodBrothers.Mission06) == 2 then
				npcHandler:say("Did you find anything of interest inside the castle?", cid)
				npcHandler.topic[cid] = 15
			else
				npcHandler:say("Enter the main hall and use the northern part of a small Old Carpet to find a Closed Trapdoor. Go downstairs and explore the castle rooms to find parts of Arthei's diary.", cid)
			end

		elseif player:getStorageValue(Storage.BloodBrothers.Questline) == 14 then
			-- Mission 6 Complete - Ask for Mission 7
			npcHandler:say("The diary reveals much about the vampire brothers. Are you ready to face them?", cid)
			npcHandler.topic[cid] = 31

		elseif player:getStorageValue(Storage.BloodBrothers.Questline) == 15 then
			-- Mission 7 - Boreth
			if player:getStorageValue(Storage.BloodBrothers.BorekthKill) == 1 then
				npcHandler:say("The plant-obsessed vampire brother Boreth... have you got proof of his death?", cid)
				npcHandler.topic[cid] = 18
			else
				npcHandler:say("Find and destroy Boreth, the plant-obsessed vampire brother in the castle.", cid)
			end

		elseif player:getStorageValue(Storage.BloodBrothers.Questline) == 16 then
			-- Mission 7 Complete - Ask for Mission 8
			npcHandler:say("One vampire brother down, three to go. Are you ready to continue hunting?", cid)
			npcHandler.topic[cid] = 32

		elseif player:getStorageValue(Storage.BloodBrothers.Questline) == 17 then
			-- Mission 8 - Lersatio
			if player:getStorageValue(Storage.BloodBrothers.LersatioKill) == 1 then
				npcHandler:say("A vain vampire hoping to see his image in the mirror once again some day... how ironic. Have you got proof of his death?", cid)
				npcHandler.topic[cid] = 17
			else
				npcHandler:say("Find and destroy Lersatio, the vain vampire brother who longs to see his reflection again.", cid)
			end

		elseif player:getStorageValue(Storage.BloodBrothers.Questline) == 18 then
			-- Mission 8 Complete - Ask for Mission 9
			npcHandler:say("Two brothers defeated! Are you ready to face Marziel?", cid)
			npcHandler.topic[cid] = 33

		elseif player:getStorageValue(Storage.BloodBrothers.Questline) == 19 then
			-- Mission 9 - Marziel
			if player:getStorageValue(Storage.BloodBrothers.MarzielKill) == 1 then
				npcHandler:say("Marziel, the tormented author of the cursed diary... have you got proof of his death?", cid)
				npcHandler.topic[cid] = 19
			else
				npcHandler:say("Have you destroyed Marziel? Remember, this mission requires a female character to complete!", cid)
			end

		elseif player:getStorageValue(Storage.BloodBrothers.Questline) == 20 then
			-- Mission 9 Complete - Ask for Mission 10
			npcHandler:say("With Marziel defeated, we're close to ending this curse. Are you ready for your final mission?", cid)
			npcHandler.topic[cid] = 25
			
		elseif player:getStorageValue(Storage.BloodBrothers.Questline) == 21 then
			-- Mission 10 - Arthei
			if player:getStorageValue(Storage.BloodBrothers.ArtheiKill) == 1 then
				npcHandler:say("Arthei, the master of the vampire brothers... have you got proof of his death?", cid)
				npcHandler.topic[cid] = 20
			else
				npcHandler:say("Have you faced Arthei, the master of the vampire brothers?", cid)
			end

		-- Quest completed
		elseif player:getStorageValue(Storage.BloodBrothers.Questline) >= 22 then
			if player:getStorageValue(Storage.BloodBrothers.RewardSelection) == -1 then
				npcHandler:say("Choose your reward: Do you want the {vampiric crest} or the {yalaharian outfit} addon?", cid)
			else
				npcHandler:say("You have already completed the Blood Brothers Quest and chosen your reward. Thank you for saving Yalahar from the vampire threat!", cid)
			end

		else
			npcHandler:say("Continue with your current mission, " .. playername .. "!", cid)
		end
	
	-- Handle eating garlic bread for Mission 1
	elseif msgcontains(msg, "garlic bread") or msgcontains(msg, "bread") then
		if player:getStorageValue(Storage.BloodBrothers.Mission01) == 1 then
		if player:getItemCount(9111) > 0 then -- Garlic bread item ID
			player:removeItem(9111, 1)
				player:setStorageValue(Storage.BloodBrothers.Questline, 2)
				player:setStorageValue(Storage.BloodBrothers.Mission01, 2) -- Mission 1 complete
				player:addExperience(1000, true)
				npcHandler:say("Very good! I can see you're definitely not a vampire. Now I can trust you with our real mission.", cid)
			else
				npcHandler:say("You don't have garlic bread! Make some by mixing flour with holy water, then using that dough on bulb of garlic, and bake it!", cid)
			end
		end
	
	-- Reward selection
	elseif msgcontains(msg, "vampiric crest") then
		if player:getStorageValue(Storage.BloodBrothers.Questline) >= 22 and player:getStorageValue(Storage.BloodBrothers.RewardSelection) == -1 then
			player:addItem(9955, 1) -- Vampiric Crest
			player:setStorageValue(Storage.BloodBrothers.RewardSelection, 1)
			npcHandler:say("Here is your vampiric crest! Wear it with pride, vampire hunter!", cid)
		end
	
	elseif msgcontains(msg, "yalaharian outfit") or msgcontains(msg, "outfit") then
		if player:getStorageValue(Storage.BloodBrothers.Questline) >= 22 and player:getStorageValue(Storage.BloodBrothers.RewardSelection) == -1 then
			player:addOutfitAddon(324, 1) -- Yalaharian Addon 1 (male)
			player:addOutfitAddon(324, 2) -- Yalaharian Addon 2 (male)
			player:addOutfitAddon(325, 1) -- Yalaharian Addon 1 (female)
			player:addOutfitAddon(325, 2) -- Yalaharian Addon 2 (female)
			player:setStorageValue(Storage.BloodBrothers.RewardSelection, 2)
			npcHandler:say("Here are your yalaharian outfit addons! You now have the full yalaharian citizen look!", cid)
		end
	
	-- Blood crystal dialogue responses
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 10 then
			npcHandler:say({
				"Oh, look how it shimmers... such a pretty sight... now we just need to get it filled with magic energy.",
				"Maybe if you could find someone ..."
			}, cid)
			npcHandler.topic[cid] = 11
			return true
		elseif npcHandler.topic[cid] == 12 then
			player:setStorageValue(Storage.BloodBrothers.Mission05, 2)
			npcHandler:say("Once you have a charged crystal, hurry back to me. I don't know how long that power will last. Good luck!", cid)
			npcHandler.topic[cid] = 0
			return true
		elseif npcHandler.topic[cid] == 13 then
			npcHandler:say({
				"Well anyway, as it seems there's more than that door protecting the castle, since you were not able to proceed any further - and those ghosts patrolling the hallway seem invulnerable. ...",
				"I wonder what the story behind this place is. Maybe you can somehow find a way past the ghosts and deeper down into the castle. ...",
				"If you could find some documents or books about this place that would be a great help. Anything that tells us more about the master of this castle and how this place got so cursed. Could you do that?"
			}, cid)
			npcHandler.topic[cid] = 14
			return true
		elseif npcHandler.topic[cid] == 14 then
			npcHandler:say("Fine. You know, those old castles sometimes have hidden passages and stuff like that. That's at least what they say in fairytales. Good luck!", cid)
			player:setStorageValue(Storage.BloodBrothers.Questline, 13) -- Advance to return phase
			player:setStorageValue(Storage.BloodBrothers.Mission06, 2) -- Advance to return phase
			npcHandler.topic[cid] = 0
			return true
		elseif npcHandler.topic[cid] == 15 then
			if player:getItemCount(1972) > 0 then -- Arthei's diary part 1
				npcHandler:say({
					"Oh! That is indeed MOST interesting. Can I have that book you found?",
					"Thank you so much, I'll grant you a small bonus for that. Let me take a closer look, hmm. There are a lot of pages missing... but that last page is kind of disturbing. ...",
					"The name on the front page says 'Marziel'... and it seems that his brothers and himself have something to do with this place. ...",
					"There are a lot of missing pages... I wonder what happened after March 30th? Listen, should you stumble across any more pages, please bring them to me for a small reward. I'd really like to figure this out. ...",
					"Apart from that, I guess to meet the brothers, you have to explore the castle even more. Maybe you can find another open door somewhere and look where - or who - it leads to?"
				}, cid)
				npcHandler.topic[cid] = 16
			else
				npcHandler:say("I need to see the diary part you found. Bring it to me!", cid)
				npcHandler.topic[cid] = 0
			end
			return true
		elseif npcHandler.topic[cid] == 16 then
			player:removeItem(1972, 1)
			player:setStorageValue(Storage.BloodBrothers.Questline, 14)
			player:setStorageValue(Storage.BloodBrothers.Mission06, 3) -- Mission 6 complete
			player:addExperience(1200, true)
			npcHandler:say("Good luck. I mean it.", cid)
			npcHandler.topic[cid] = 0
			return true
		elseif npcHandler.topic[cid] == 17 then
			player:setStorageValue(Storage.BloodBrothers.Questline, 18)
			player:setStorageValue(Storage.BloodBrothers.Mission08, 3) -- Mission 8 complete
			player:addExperience(2400, true)
			npcHandler:say("You are definitely one of the bravest and craziest adventurers I ever met. Each time you prove to me that there is no task too dangerous for you.", cid)
			npcHandler:say("There are only two brothers left now, and I can feel that their grasp on Yalahar is getting weaker. We cannot stop now but have to finish what we started.", cid)
			npcHandler.topic[cid] = 0
			return true
		elseif npcHandler.topic[cid] == 18 then
			player:setStorageValue(Storage.BloodBrothers.Questline, 16)
			player:setStorageValue(Storage.BloodBrothers.Mission07, 3) -- Mission 7 complete
			player:addExperience(1000, true)
			npcHandler:say("That's what I was hoping for! I will start investigating that dust. Maybe we can gain valuable information on how we can defeat the vampire plague once and for all.", cid)
			npcHandler.topic[cid] = 0
			return true
		elseif npcHandler.topic[cid] == 19 then
			player:setStorageValue(Storage.BloodBrothers.Questline, 20)
			player:setStorageValue(Storage.BloodBrothers.Mission09, 3) -- Mission 9 complete
			player:addExperience(3600, true)
			npcHandler:say("The author of that cursed diary is finally destroyed! Now only Arthei remains - their master!", cid)
			npcHandler.topic[cid] = 0
			return true
		elseif npcHandler.topic[cid] == 20 then
			player:setStorageValue(Storage.BloodBrothers.Questline, 22)
			player:setStorageValue(Storage.BloodBrothers.Mission10, 3) -- Mission 10 complete
			player:addExperience(1000, true)
			player:addItem(9447, 1) -- Blood Goblet
			npcHandler:say("Incredible! You have destroyed all four vampire brothers and lifted their curse from these lands!", cid)
			npcHandler:say("As a reward, choose your prize: Do you want the {vampiric crest} or the {yalaharian outfit} addon?", cid)
			npcHandler.topic[cid] = 0
			return true
		elseif npcHandler.topic[cid] == 25 then
			player:setStorageValue(Storage.BloodBrothers.Mission10, 1) -- Start Mission 10
			player:setStorageValue(Storage.BloodBrothers.Questline, 21)
			npcHandler:say("Face Arthei, the leader of the vampire brothers. End this curse once and for all!", cid)
			npcHandler.topic[cid] = 0
			return true
		elseif npcHandler.topic[cid] == 26 then
			player:setStorageValue(Storage.BloodBrothers.Mission02, 1) -- Start Mission 2
			player:setStorageValue(Storage.BloodBrothers.GarlicCookieCount, 0) -- Initialize cookie count for quest log
			player:setStorageValue(Storage.BloodBrothers.Questline, 3)
			npcHandler:say("Yalahar is being infiltrated by vampires. I need you to help identify the suspects!", cid)
			npcHandler:say("I need you to find vampire suspects by giving garlic cookies to citizens of Yalahar. Bake garlic cookies and offer them to Serafin, Lisander, Ortheus, Maris, and Armenius.", cid)
			npcHandler:say("Watch their reactions carefully and remember who refuses or reacts suspiciously!", cid)
			npcHandler.topic[cid] = 0
			return true
		elseif npcHandler.topic[cid] == 27 then
			player:setStorageValue(Storage.BloodBrothers.Mission03, 1) -- Start Mission 3
			player:setStorageValue(Storage.BloodBrothers.Questline, 4)
			npcHandler:say("Now I need you to cast 'alori mort' on them to reveal their true nature.", cid)
			npcHandler:say("Go to Armenius specifically - he's the most suspicious. Say the spell while facing him directly!", cid)
			npcHandler.topic[cid] = 0
			return true
		elseif npcHandler.topic[cid] == 28 then
			player:setStorageValue(Storage.BloodBrothers.Mission04, 1) -- Start Mission 4
			player:setStorageValue(Storage.BloodBrothers.Questline, 7)
			npcHandler:say("Find Harlow at the docks and ask him for passage to Vengoth. Explore the vampire lands and find a way into their castle!", cid)
			npcHandler.topic[cid] = 0
			return true
		elseif npcHandler.topic[cid] == 29 then
			player:setStorageValue(Storage.BloodBrothers.Mission05, 1) -- Start Mission 5
			player:setStorageValue(Storage.BloodBrothers.Questline, 9)
			npcHandler:say("You need to find a way past those ghostly guardians. Look for hidden passages or another entrance!", cid)
			npcHandler.topic[cid] = 0
			return true
		elseif npcHandler.topic[cid] == 30 then
			player:setStorageValue(Storage.BloodBrothers.Mission06, 1) -- Start Mission 6
			player:setStorageValue(Storage.BloodBrothers.Questline, 12)
			npcHandler:say("Now that you're deeper in the castle, your next mission is to find information about the vampire brothers.", cid)
			npcHandler:say("Enter the main hall and use the northern part of a small Old Carpet to find a Closed Trapdoor. Go downstairs and explore the castle rooms to find parts of Arthei's diary.", cid)
			npcHandler.topic[cid] = 0
			return true
		elseif npcHandler.topic[cid] == 31 then
			player:setStorageValue(Storage.BloodBrothers.Mission07, 1) -- Start Mission 7
			player:setStorageValue(Storage.BloodBrothers.Questline, 15)
			npcHandler:say("Find and destroy Boreth, the plant-obsessed vampire brother in the castle.", cid)
			npcHandler.topic[cid] = 0
			return true
		elseif npcHandler.topic[cid] == 32 then
			player:setStorageValue(Storage.BloodBrothers.Mission08, 1) -- Start Mission 8
			player:setStorageValue(Storage.BloodBrothers.Questline, 17)
			npcHandler:say("Listen, brave vampire slayer, I don't think that your task in this castle is done yet. According to the diary you found, there are three other brothers called Lersatio, Marziel and Arthei.", cid)
			npcHandler:say("We have to seek them all out and destroy them in order to weaken their power over the land. After Boreth's death, it is quite possible that you can gain access to another tower in the castle.", cid)
			npcHandler:say("That is your chance to find the second brother and awaken him. Good luck - again.", cid)
			npcHandler.topic[cid] = 0
			return true
		elseif npcHandler.topic[cid] == 33 then
			player:setStorageValue(Storage.BloodBrothers.Mission09, 1) -- Start Mission 9
			player:setStorageValue(Storage.BloodBrothers.Questline, 19)
			npcHandler:say("See if you can slip into another tower of the castle and climb up to the room of the third brother. Since Arthei is their master, I guess Marziel is who we are going for now.", cid)
			npcHandler:say("The author of that diary... writing down the cursed story of his life. I hope he will rest in peace. Good luck.", cid)
			npcHandler.topic[cid] = 0
			return true
		elseif npcHandler.topic[cid] == 21 then
			player:setStorageValue(Storage.BloodBrothers.Questline, 4)
			player:setStorageValue(Storage.BloodBrothers.Mission02, 3) -- Mark as completed for quest log
			player:setStorageValue(Storage.BloodBrothers.GarlicCookieCount, 6) -- Mark quest complete in log
			npcHandler:say("Excellent work! I knew there were vampire suspects among us.", cid)
			npcHandler.topic[cid] = 0
			return true
		elseif npcHandler.topic[cid] == 22 then
			player:setStorageValue(Storage.BloodBrothers.Questline, 5)
			player:setStorageValue(Storage.BloodBrothers.Mission03, 3) -- Mark as completed for quest log
			player:setStorageValue(Storage.BloodBrothers.VengothAccess, 1) -- Allow travel to Vengoth
			player:addItem(9117, 1) -- Julius' map
			npcHandler:say("So Armenius IS a vampire! This is worse than I thought. Now we must travel to the source - the dark lands of Vengoth.", cid)
			npcHandler.topic[cid] = 0
			return true
		elseif npcHandler.topic[cid] == 23 then
			player:setStorageValue(Storage.BloodBrothers.Questline, 6)
			player:setStorageValue(Storage.BloodBrothers.Mission04, 3) -- Mark as completed for quest log
			npcHandler:say("The castle is heavily protected by ghosts? This gets more interesting by the moment.", cid)
			npcHandler.topic[cid] = 0
			return true
		elseif npcHandler.topic[cid] == 24 then
			player:setStorageValue(Storage.BloodBrothers.Questline, 8)
			player:setStorageValue(Storage.BloodBrothers.Mission05, 5) -- Mission 5 complete
			npcHandler:say("Excellent! The ritual worked perfectly. Now that you're deeper in the castle, we can continue our investigation.", cid)
			npcHandler.topic[cid] = 0
			return true
		end
	elseif msgcontains(msg, "someone") or msgcontains(msg, "lost") or msgcontains(msg, "dear") then
		if npcHandler.topic[cid] == 11 then
			npcHandler:say("If you can find someone like that, I'm sure that you will be able to charge the blood crystal. That's the only help I can give you though. Are you willing to try?", cid)
			npcHandler.topic[cid] = 12
			return true
		end

	-- Info keywords
	elseif msgcontains(msg, "yalahar") then
		npcHandler:say("Yalahar, our great city, is under attack by vampires. We must protect it at all costs!", cid)

	elseif msgcontains(msg, "vampire") then
		npcHandler:say("Vampires are evil undead creatures that feed on blood. They have infiltrated our city and must be stopped!", cid)

	elseif msgcontains(msg, "vengoth") then
		npcHandler:say("Vengoth is the dark homeland of the vampires. A cursed place where no living soul should venture... but we have no choice.", cid)

	elseif msgcontains(msg, "harlow") then
		npcHandler:say("Harlow is a ship captain who can take you to Vengoth. You'll find him at the docks here in Yalahar.", cid)

	end

	return true
end

npcHandler:setMessage(MESSAGE_GREET, "Greetings, |PLAYERNAME|! I fight against the vampire threat plaguing Yalahar. Do you want to help with my {mission}?")
npcHandler:setMessage(MESSAGE_FAREWELL, "May the light protect you from the creatures of darkness, |PLAYERNAME|!")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Stay safe out there!")

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 12405, buy = 0, sell = 320, subType = 0, name = "blood preservation"},
    {id = 10602, buy = 0, sell = 275, subType = 0, name = "vampire teeth"},
    {id = 2692, buy = 10, sell = 0, subType = 0, name = "flour"},
    {id = 7494, buy = 300, sell = 0, subType = 0, name = "holy water"},
    {id = 9114, buy = 15, sell = 0, subType = 0, name = "bulb of garlic"},
}

-- Helper function to find shop item by id and subType (for fluid containers)
local function getShopItem(itemId, subType, isBuying)
    local itemType = ItemType(itemId)
    if itemType:isFluidContainer() then
        for _, item in ipairs(shopItems) do
            if item.id == itemId and item.subType == subType then
                return item
            end
        end
    end
    -- For non-fluid items, find the entry that matches the operation
    for _, item in ipairs(shopItems) do
        if item.id == itemId then
            if isBuying and item.buy > 0 then
                return item
            elseif not isBuying and item.sell > 0 then
                return item
            end
        end
    end
    -- Fallback to first match
    for _, item in ipairs(shopItems) do
        if item.id == itemId then
            return item
        end
    end
    return nil
end

local function openTradeWindow(cid, message, keywords, parameters, node)
    if not npcHandler:isFocused(cid) then return false end
    local player = Player(cid)
    if not player then return false end
    local npc = Npc(getNpcCid())
    local shopList = {}
    for _, item in ipairs(shopItems) do
        table.insert(shopList, {id = item.id, buy = item.buy, sell = item.sell, subType = item.subType or 0, name = item.name})
    end
    npc:openShopWindow(player, shopList, function() return true end, function() return true end)
    npcHandler:say('Take all the time you need to browse my wares.', cid)
    return true
end
keywordHandler:addKeyword({'trade'}, openTradeWindow, {npcHandler = npcHandler})


-- NpcType callbacks (MUST call setCurrentNpc first!)
npcType:eventType(NPCS_EVENT_APPEAR)
npcType:onAppear(function(npc, creature)
    setCurrentNpc(npc)
    npcHandler:onCreatureAppear(creature)
end)

npcType:eventType(NPCS_EVENT_DISAPPEAR)
npcType:onDisappear(function(npc, creature)
    setCurrentNpc(npc)
    npcHandler:onCreatureDisappear(creature)
end)

npcType:eventType(NPCS_EVENT_SAY)
npcType:onSay(function(npc, creature, type, message)
    setCurrentNpc(npc)
    npcHandler:onCreatureSay(creature, type, message)
end)

npcType:eventType(NPCS_EVENT_THINK)
npcType:onThink(function(npc, interval)
    setCurrentNpc(npc)
    npcHandler:onThink()
end)

npcType:eventType(NPCS_EVENT_CLOSECHANNEL)
npcType:onCloseChannel(function(npc, creature)
    setCurrentNpc(npc)
    npcHandler:onPlayerCloseChannel(creature)
end)

npcType:eventType(NPCS_EVENT_BUYITEM)
npcType:onBuyItem(function(npc, player, itemId, subType, amount, ignoreCap, inBackpacks)
    local shopItem = getShopItem(itemId, subType, true)
    if not shopItem or shopItem.buy <= 0 then return false end
    local totalCost = amount * shopItem.buy
    if player:getTotalMoney() < totalCost then
        player:sendCancelMessage("You don't have enough money.")
        return false
    end
    local itemSubType = shopItem.subType or 1
    local bought = doNpcSellItem(player:getId(), itemId, amount, itemSubType, ignoreCap, inBackpacks, ITEM_BACKPACK)
    if bought == 0 then
        player:sendCancelMessage("You do not have enough capacity.")
        return false
    end
    player:removeTotalMoney(bought * shopItem.buy)
    player:sendTextMessage(MESSAGE_INFO_DESCR, "Bought " .. bought .. "x " .. shopItem.name .. " for " .. (bought * shopItem.buy) .. " gold.")
    return true
end)

npcType:eventType(NPCS_EVENT_SELLITEM)
npcType:onSellItem(function(npc, player, itemId, subType, amount, ignoreEquipped)
    local shopItem = getShopItem(itemId, subType, false)
    if not shopItem or shopItem.sell <= 0 then return false end
    local totalPrice = amount * shopItem.sell
    local itemName = shopItem.name or ItemType(itemId):getName()
    
    local itemSubType = -1
    if ItemType(itemId):isFluidContainer() then
        itemSubType = subType
    end
    
    if doPlayerSellItem(player:getId(), itemId, amount, totalPrice, itemSubType, ignoreEquipped) then
        player:sendTextMessage(MESSAGE_INFO_DESCR, "Sold " .. amount .. "x " .. shopItem.name .. " for " .. (amount * shopItem.sell) .. " gold.")
        return true
    end
    player:sendCancelMessage("You do not have this object.")
    return false
end)

npcHandler:addModule(FocusModule:new())
npcType:register()
