-- Curos - Converted from XML to Lua NpcType
-- Original XML: data/npc/Curos.xml
-- Original Script: data/npc/scripts/Curos.lua

local npcName = "Curos"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a curos")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 29})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function hasActiveTask(player)
	local tasks = {
		Storage.AnUneasyAllianceTasks.MilkDelivery,
		Storage.AnUneasyAllianceTasks.FeedBeast,
		Storage.AnUneasyAllianceTasks.HonourDead,
		Storage.AnUneasyAllianceTasks.FoulSpirits
	}

	for _, taskStorage in ipairs(tasks) do
		if player:getStorageValue(taskStorage) == 1 then
			return true -- Player has an active task
		end
	end
	return false -- No active tasks
end

local function canGetDailyTask(player)
	local resetTime = player:getStorageValue(Storage.AnUneasyAllianceTasks.DailyTaskReset)
	if resetTime <= 0 then
		return true -- Never done a daily task before
	end

	-- Check if 20 hours (72000 seconds) have passed since last task completion
	return os.time() >= resetTime
end

local function getRandomTask(player)
	local tasks = {
		{storage = Storage.AnUneasyAllianceTasks.MilkDelivery, name = "Milk Delivery"},
		{storage = Storage.AnUneasyAllianceTasks.FeedBeast, name = "To Feed a Beast"},
		{storage = Storage.AnUneasyAllianceTasks.HonourDead, name = "Honour The Dead"},
		{storage = Storage.AnUneasyAllianceTasks.FoulSpirits, name = "Foul Spirits"}
	}

	-- Filter out tasks that are already completed (value 2) or in progress (value 1)
	local availableTasks = {}
	for _, task in ipairs(tasks) do
		if player:getStorageValue(task.storage) < 1 then
			table.insert(availableTasks, task)
		end
	end

	if #availableTasks == 0 then
		return nil -- All tasks completed
	end

	return availableTasks[math.random(#availableTasks)]
end

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	if(msgcontains(msg, "mission")) then
		if(player:getStorageValue(Storage.TheNewFrontier.Questline) == 17) then
			npcHandler:say("You come here to ask us to spare your people? This land has no tolerance for the weak, we have it neither. If you want us to consider you as useful for us, you'll have to prove it in a {test} of strength and courage. ", cid)
			npcHandler.topic[cid] = 1
		elseif(player:getStorageValue(Storage.TheNewFrontier.Questline) == 19) then
			npcHandler:say({
				"We have seen that you can fight and survive. Yet, it will also need cleverness and courage to survive in these lands. We might see later if you've got what it takes. ...",
				"However, I stand to my word - our hordes will spare your insignificant piece of rock for now. Time will tell if you are worthy living next to us. ...",
				"Still, it will take years until we might consider you as an ally, but this is a start at least."
			}, cid)
			player:setStorageValue(Storage.TheNewFrontier.Questline, 20)
			player:setStorageValue(Storage.TheNewFrontier.Mission06, 3) --Questlog, The New Frontier Quest "Mission 06: Days Of Doom"
			
		elseif(player:getStorageValue(Storage.AnUneasyAlliance) == -1) then
			--An Uneasy Alliance
			player:setStorageValue(Storage.AnUneasyAlliance, 1)
			player:registerEvent("UneasyAllianceRenegadeOrc") -- Dynamic registration for renegade orc events 
			npcHandler:say({
					'So you still think you can be of any use for us? Words are cheap and easy. Admittedly, you\'ve passed our first test but even some resilient beast might have accomplished that. ...',
					'Your actions will tell if you are only yelping for attention like a puppy or if you have the teeth of a wolf. ...',
					'A first tiny step was taken. You survived the test and ensured the survival of your allies for a while. Now it is time to make the next step. ...',
					'So listen human: Our rule over the orcs is not unchallenged. Of course now and then someone shows up who thinks he can defeat us. Usually these fights end fast and bloody in the ring. ...',
					'Right now, some coward from our midst, who is too afraid to face us in single combat, has gathered a group of followers, hoping more will follow and change sides. ...',
					'With your help, his defeat will not only be deadly but also humiliating and so discourage others to follow his example. ...',
					'You will seek out this rebel commander in his hideout and kill him. We will show them that not even a Mooh\'Tah master is needed to get rid of such wannabe leaders but that a mere human can handle them. ...',
					'Find him in the mountain north-west of here and kill him. If you find any loot, you can keep it.'
				}, cid)
		elseif(player:getStorageValue(Storage.AnUneasyAlliance) == 2) then
			player:setStorageValue(Storage.AnUneasyAlliance, 3)
			npcHandler:say({
					'Finally, our enemy\'s vision is obscured. Now we can move in for some more daring raids until they replace their scrying device. You have proven yourself brave and useful so far. With that, you bought your allies some more days to live. ...',
					'Here is a reward. It\'s a strange tome that we\'ve found in the lizard ruins. Maybe it is of some value for you or your allies.'
			}, cid)
			doPlayerAddItem(player,11134,1)
		elseif(player:getStorageValue(Storage.AnUneasyAlliance) == 3) then
			npcHandler:say({
					'You have proven yourself capable of handling our enemies. But there is more work to be done. Our scouts have discovered a magical scrying device in the lizard tower that allows our enemies to spy on our movements. ...',
					'You must destroy this device. Find the lizard tower and eliminate this threat. Return to me once the device is destroyed.'
			}, cid)
		elseif(player:getStorageValue(Storage.AnUneasyAlliance) == 4) then
			player:setStorageValue(Storage.AnUneasyAlliance, 5)
			npcHandler:say({
					'You have destroyed the scrying device! Our enemies are now blind to our movements. This is a significant victory for us. ...',
					'With this accomplishment, you have proven yourself worthy. Our hordes will spare your people for now. But remember, this is just the beginning.'
			}, cid)
		elseif(player:getStorageValue(Storage.AnUneasyAlliance) == 5) then
			npcHandler:say("There is nothing of urgency to be done right now. But you may improve your status by doing some minor tasks to show your courage and dedication. Are you ready for that?", cid)
			npcHandler.topic[cid] = 2
		end			
	elseif(msgcontains(msg, "test")) then
		if(npcHandler.topic[cid] == 1) then
			npcHandler:say({
				"First we will test your strength and endurance. You'll have to face one of the most experienced Mooh'Tah masters. As you don't stand a chance to beat such an opponent, your test will be simply to survive. ...",
				"Face him in a battle and survive for two minutes. If you do, we will be willing to assume that your are prepared for the life in these lands. Enter the ring of battle, close to my quarter. Return to me after you have passed this test."
			}, cid)
			npcHandler.topic[cid] = 0
			player:setStorageValue(Storage.TheNewFrontier.Questline, 18)
			player:setStorageValue(Storage.TheNewFrontier.Mission06, 2) --Questlog, The New Frontier Quest "Mission 06: Days Of Doom"
		end
	elseif(msgcontains(msg, "yes")) then
		if(npcHandler.topic[cid] == 2) then
			-- First check if player already has an active task
			if hasActiveTask(player) then
				npcHandler:say("You already have an active task. Complete it first before asking for another one.", cid)
				npcHandler.topic[cid] = 0
				return true
			end

			-- Then check daily reset timer
			if not canGetDailyTask(player) then
				npcHandler:say("I don't need your help for now. Come back later when you can assist me again.", cid)
				npcHandler.topic[cid] = 0
				return true
			end

			local task = getRandomTask(player)
			if task then
				player:setStorageValue(task.storage, 1)
				if task.name == "Milk Delivery" then
					npcHandler:say({
						"Sometimes even a Mooh'Tah master gets sentimental. We rarely have the opportunity to drink some milk here in this strange land. ...",
						"As you travel a lot, you might have the opportunity to get some. So bring me a bucket full of milk!"
					}, cid)
				elseif task.name == "To Feed a Beast" then
					npcHandler:say({
						"We are breeding more and more wolves for our elite cavalry. Unfortunately the orcs have killed nearly all giant caimans that lived here with the lizards. ...",
						"Those vile predators delivered the finest meat to raise the most fierce war wolves. These days such food has become rare. At least the leader of the wolf pack needs some caiman steaks now and then. ...",
						"In the lizard territory you'll find some remaining caimans. Take an obsidian knife and slice a caiman steak from one of their corpses."
					}, cid)
				elseif task.name == "Honour The Dead" then
					npcHandler:say({
						"Some time ago, we pursued the lizards through the mountains. Sadly, one of our youngest members led his troop into an ambush and was slain. ...",
						"The survivors buried him, but only rarely someone of us manages to visit his grave to honour him. ...",
						"You'll find his grave in the mountains north of here. His grave is in the western part of the mountains at the shore. Meditate on his grave about his failure to honour him."
					}, cid)
				elseif task.name == "Foul Spirits" then
					npcHandler:say({
						"While we have driven the lizards back behind the mountains, there are still things that simply can't be fought. When the lizards fled, they left some evil spirits behind. ...",
						"These spirits haunt our orcs and drive them mad. It will be your task to appease a tree spirit in an infested grove to the east. ...",
						"You will need a wooden flute. Travel to the haunted trees in the east, close to the shore as well as close to the mountain. ...",
						"Stand on the sqm in front of the tree spirit when using the flute. There's a chance of failing - if you fail, creatures will spawn that you must defeat. ...",
						"Wait for the tree spirit to calm down between attempts, as you can't appease it while it has its visible form."
					}, cid)
				end
			else
				npcHandler:say("You have completed all available tasks for now. Return later when new tasks become available.", cid)
			end
			npcHandler.topic[cid] = 0
		end
	elseif(msgcontains(msg, "task")) then
		-- Check if player has an active task first
		if hasActiveTask(player) then
			local milkDelivery = player:getStorageValue(Storage.AnUneasyAllianceTasks.MilkDelivery)
			local feedBeast = player:getStorageValue(Storage.AnUneasyAllianceTasks.FeedBeast)
			local honourDead = player:getStorageValue(Storage.AnUneasyAllianceTasks.HonourDead)
			local foulSpirits = player:getStorageValue(Storage.AnUneasyAllianceTasks.FoulSpirits)

			if milkDelivery == 1 then
			local bucket = player:getItemById(2006, true)
			if bucket and bucket:getAttribute(ITEM_ATTRIBUTE_CHARGES) == 12 then
				bucket:remove(1)
				player:setStorageValue(Storage.AnUneasyAllianceTasks.MilkDelivery, 2)
				player:setStorageValue(Storage.AnUneasyAllianceTasks.DailyTaskReset, os.time() + 20*60*60) -- 20 hours
				player:addMoney(500)
				player:addExperience(800, true)
				npcHandler:say("Hmm, milk. The source of all life! Here is your reward.", cid)
			else
				npcHandler:say("Do you have a bucket of milk for me?", cid)
			end
		elseif feedBeast == 1 then
			if player:getItemCount(10297) >= 1 then
				player:removeItem(10297, 1)
				player:setStorageValue(Storage.AnUneasyAllianceTasks.FeedBeast, 2)
				player:setStorageValue(Storage.AnUneasyAllianceTasks.DailyTaskReset, os.time() + 20*60*60) -- 20 hours
				player:addMoney(750)
				player:addExperience(1800, true)
				npcHandler:say("Excellent. This will serve well to satisfy the hunger of the pack leader at least for now. Here is your reward.", cid)
			else
				npcHandler:say("Did you bring me a caiman steak?", cid)
			end
		elseif honourDead == 1 then
			npcHandler:say("You haven't honoured the dead yet. Find the orc grave in the mountains north of here and meditate there.", cid)
		elseif honourDead == 2 then
			player:setStorageValue(Storage.AnUneasyAllianceTasks.DailyTaskReset, os.time() + 20*60*60) -- 20 hours
			player:addMoney(1000)
			player:addExperience(1100, true)
			npcHandler:say("Though I don't feel the contemplation of true meditation in you, I guess it's the best I can expect of a human. Here is a small reward for your efforts.", cid)
		elseif foulSpirits == 1 then
			player:setStorageValue(Storage.AnUneasyAllianceTasks.FoulSpirits, 2)
			player:setStorageValue(Storage.AnUneasyAllianceTasks.DailyTaskReset, os.time() + 20*60*60) -- 20 hours
			player:addMoney(1000)
			player:addExperience(1400, true)
			npcHandler:say("So the spirits have found some peace for a while. And you have earned yourself a reward.", cid)
			else
				npcHandler:say("You don't have any active tasks right now. Ask me about a {mission} if you'd like to help us.", cid)
			end
		else
			npcHandler:say("You don't have any active tasks right now. Ask me about a {mission} if you'd like to help us.", cid)
		end
	end
	return true
end

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


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

npcHandler:addModule(FocusModule:new())
npcType:register()
