-- Malor - Converted from XML to Lua NpcType
-- Original XML: data/npc/Malor.xml
-- Original Script: data/npc/scripts/Malor.lua

local npcName = "Malor"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a malor")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 103})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function releasePlayer(cid)
	if not Player(cid) then
		return
	end

	npcHandler:releaseFocus(cid)
	npcHandler:resetNpc(cid)
end

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	local missionProgress = player:getStorageValue(Storage.DjinnWar.EfreetFaction.Mission03) or -1
	if msgcontains(msg, 'mission') then
		if (player:getStorageValue(Storage.DjinnWar.EfreetFaction.Mission02) or -1) == 3 then
			if missionProgress < 1 then
				npcHandler:say({
					'I guess this is the first time I entrust a human with a mission. And such an important mission, too. But well, we live in hard times, and I am a bit short of adequate staff. ...',
					'Besides, Baa\'leal told me you have distinguished yourself well in previous missions, so I think you might be the right person for the job. ...',
					'But think carefully, human, for this mission will bring you close to certain death. Are you prepared to embark on this mission?'
				}, cid)
				npcHandler.topic[cid] = 1

			elseif missionProgress == 1 then
				npcHandler:say('You haven\'t finished your final mission yet. Shall I explain it again to you?', cid)
				npcHandler.topic[cid] = 1

			elseif missionProgress == 2 then
				npcHandler:say('Have you found Fa\'hradin\'s lamp and placed it in Malor\'s personal chambers?', cid)
				npcHandler.topic[cid] = 2
			else
				npcHandler:say('There\'s no mission left for you, friend of the Efreet. However, I have a {task} for you.', cid)
			end
		else
			npcHandler:say({
				'So you would like to fight for us. Hmm. ...',
				'You show true courage, human, but I will not accept your offer at this point of time.'
			}, cid)
		end

	elseif npcHandler.topic[cid] == 1 then
		if msgcontains(msg, 'yes') then
			npcHandler:say({
				'Well, listen. We are trying to acquire the ultimate weapon to defeat Gabel: Fa\'hradin\'s lamp! ...',
				'At the moment it is still in the possession of that good old friend of mine, the Orc King, who kindly released me from it. ...',
				'However, for some reason he is not as friendly as he used to be. You better watch out, human, because I don\'t think you will get the lamp without a fight. ...',
				'Once you have found the lamp you must enter Ashta\'daramai again. Sneak into Gabel\'s personal chambers and exchange his sleeping lamp with Fa\'hradin\'s lamp! ...',
				'If you succeed, the war could be over one night later!'
			}, cid)
			player:setStorageValue(Storage.DjinnWar.EfreetFaction.Mission03, 1)

		elseif msgcontains(msg, 'no') then
			npcHandler:say('Your choice.', cid)
			npcHandler.topic[cid] = 0
		end

	elseif npcHandler.topic[cid] == 2 then
		if msgcontains(msg, 'yes') then
			npcHandler:say({
				'Well well, human. So you really have made it - you have smuggled the modified lamp into Gabel\'s bedroom! ...',
				'I never thought I would say this to a human, but I must confess I am impressed. ...',
				'Perhaps I have underestimated you and your kind after all. ...',
				'I guess I will take this as a lesson to keep in mind when I meet you on the battlefield. ...',
				'But that\'s in the future. For now, I will confine myself to give you the permission to trade with my people whenever you want to. ...',
				'Farewell, human!'
			}, cid)
			player:setStorageValue(Storage.DjinnWar.EfreetFaction.Mission03, 3)
			player:setStorageValue(Storage.DjinnWar.EfreetFaction.DoorToMaridTerritory, 1)
			player:addAchievement('Efreet Ally')
			addEvent(releasePlayer, 1000, cid)

		elseif msgcontains(msg, 'no') then
			npcHandler:say('Just do it!', cid)
		end
		npcHandler.topic[cid] = 0
	end
	return true
end

npcHandler:setMessage(MESSAGE_GREET, 'Greetings, human |PLAYERNAME|. My patience with your kind is limited, so speak quickly and choose your words well.')
npcHandler:setMessage(MESSAGE_FAREWELL, 'Farewell, human. When I have taken my rightful place I shall remember those who served me well. Even if they are only humans.')
npcHandler:setMessage(MESSAGE_WALKAWAY, 'Farewell, human.')

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)

local focusModule = FocusModule:new()
focusModule:addGreetMessage('hi')
focusModule:addGreetMessage('hello')
focusModule:addGreetMessage('djanni\'hah')
npcHandler:addModule(focusModule)


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

npcType:register()
