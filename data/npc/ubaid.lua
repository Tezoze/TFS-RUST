-- Ubaid - Converted from XML to Lua NpcType
-- Original XML: data/npc/Ubaid.xml
-- Original Script: data/npc/scripts/Ubaid.lua

local npcName = "Ubaid"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a ubaid")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 51})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local condition = Condition(CONDITION_FIRE)
condition:setParameter(CONDITION_PARAM_DELAYED, 1)
condition:addDamage(150, 2000, -10)

local function greetCallback(cid, message)
	local player = Player(cid)
	if not player:getCondition(CONDITION_FIRE) and message and type(message) == "string" and not msgcontains(string.lower(message), 'djanni\'hah') and player:getStorageValue(Storage.DjinnWar.Faction.Efreet) ~= 1 then
		player:getPosition():sendMagicEffect(CONST_ME_EXPLOSIONAREA)
		player:addCondition(condition)
		npcHandler:say('Take this!', cid)
		return false
	end

	if player:getStorageValue(Storage.DjinnWar.Faction.Greeting) == -1 then
		-- Manually parse the message to replace |PLAYERNAME|
		local playerName = player:getName()
		local messages = {
			'Hahahaha! ...',
			playerName .. ', that almost sounded like the word of greeting. Humans - cute they are!'
		}
		npcHandler:say(messages, cid)
		return false
	end

	if player:getStorageValue(Storage.DjinnWar.Faction.Efreet) ~= 1 then
		npcHandler:setMessage(MESSAGE_GREET, 'What? You know the word, |PLAYERNAME|? All right then - I won\'t kill you. At least, not now.  What brings you {here}?')
	else
		npcHandler:setMessage(MESSAGE_GREET, 'Still alive, |PLAYERNAME|? What brings you {here}?')
	end
	return true
end

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)

	-- To Appease the Mighty Quest
	if msgcontains(msg, "mission") and player:getStorageValue(Storage.TibiaTales.ToAppeaseTheMightyQuest) == 2 then
			npcHandler:say({
				'You have the smell of the Marid on you. Tell me who sent you?'}, cid)
			npcHandler.topic[cid] = 9
			elseif msgcontains(msg, "kazzan") and npcHandler.topic[cid] == 9 then
			npcHandler:say({
				'And he is sending a worm like you to us!?! The mighty Efreet!! Tell him that we won\'t be part in his \'great\' plans and now LEAVE!! ...',
				'...or do you want to join us and fight those stinking Marid who claim themselves to be noble and righteous?!? Just let me know.'}, cid)
			player:setStorageValue(Storage.TibiaTales.ToAppeaseTheMightyQuest, player:getStorageValue(Storage.TibiaTales.ToAppeaseTheMightyQuest) + 1)
	end

	if msgcontains(msg, 'passage') then
		if player:getStorageValue(Storage.DjinnWar.Faction.Efreet) ~= 1 then
			npcHandler:say({
				'Only the mighty Efreet, the true djinn of Tibia, may enter Mal\'ouquah! ...',
				'All Marid and little worms like yourself should leave now or something bad may happen. Am I right?'
			}, cid)
			npcHandler.topic[cid] = 1
		else
			npcHandler:say('You already pledged loyalty to king Malor!', cid)
		end

	elseif msgcontains(msg, 'here') then
			npcHandler:say({
				'Only the mighty Efreet, the true djinn of Tibia, may enter Mal\'ouquah! ...',
				'All Marid and little worms like yourself should leave now or something bad may happen. Am I right?'
			}, cid)
			npcHandler.topic[cid] = 1

	elseif npcHandler.topic[cid] == 1 then
		if msgcontains(msg, 'yes') then
			npcHandler:say('Of course. Then don\'t waste my time and shove off.', cid)
			npcHandler.topic[cid] = 0

		elseif msgcontains(msg, 'no') then
			if player:getStorageValue(Storage.DjinnWar.Faction.Marid) == 1 then
				npcHandler:say('Who do you think you are? A Marid? Shove off you worm!', cid)
				npcHandler.topic[cid] = 0
			else
				npcHandler:say({
					'Of cour... Huh!? No!? I can\'t believe it! ...',
					'You... you got some nerves... Hmm. ...',
					'Maybe we have some use for someone like you. Would you be interested in working for us. Helping to fight the Marid?'
				}, cid)
				npcHandler.topic[cid] = 2
			end
		end

	elseif npcHandler.topic[cid] == 2 then
		if msgcontains(msg, 'yes') then
			npcHandler:say('So you pledge loyalty to king Malor and you are willing to never ever set foot on Marid\'s territory, unless you want to kill them? Yes?', cid)
			npcHandler.topic[cid] = 3

		elseif msgcontains(msg, 'no') then
			npcHandler:say('Of course. Then don\'t waste my time and shove off.', cid)
			npcHandler.topic[cid] = 0
		end

	elseif npcHandler.topic[cid] == 3 then
		if msgcontains(msg, 'yes') then
			npcHandler:say({
				'Well then - welcome to Mal\'ouquah. ...',
				'Go now to general Baa\'leal and don\'t forget to greet him correctly! ...',
				'And don\'t touch anything!'
			}, cid)
			player:setStorageValue(Storage.DjinnWar.EfreetFaction.Start, 1)
			player:setStorageValue(Storage.DjinnWar.Faction.Efreet, 1)
			player:setStorageValue(Storage.DjinnWar.Faction.Greeting, 0)
			player:setStorageValue(Storage.Factions, 2)

		elseif msgcontains(msg, 'no') then
			npcHandler:say('Of course. Then don\'t waste my time and shove off.', cid)
		end
		npcHandler.topic[cid] = 0
	end
	return true
end

npcHandler:setMessage(MESSAGE_FAREWELL, 'Farewell human!')
npcHandler:setMessage(MESSAGE_WALKAWAY, 'Farewell human!')

npcHandler:setCallback(CALLBACK_GREET, greetCallback)
npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)

local focusModule = FocusModule:new()
focusModule:addGreetMessage('hi')
focusModule:addGreetMessage('hello')
focusModule:addGreetMessage('djanni\'hah')
focusModule:addGreetMessage('DJANNI\'HAH')
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
