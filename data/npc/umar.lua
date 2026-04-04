-- Umar - Converted from XML to Lua NpcType
-- Original XML: data/npc/Umar.xml
-- Original Script: data/npc/scripts/Umar.lua

local npcName = "Umar"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a umar")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 80})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function greetCallback(cid, message)
	local player = Player(cid)
	if message and type(message) == "string" and not msgcontains(message, 'djanni\'hah') and player:getStorageValue(Storage.DjinnWar.Faction.Marid) ~= 1 then
		-- Manually parse the message to replace |PLAYERNAME|
		local playerName = player:getName()
		local msg = 'Whoa! A human! This is no place for you, ' .. playerName .. '. Go and play somewhere else.'
		npcHandler:say(msg, cid)
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

	if player:getStorageValue(Storage.DjinnWar.Faction.Marid) ~= 1 then
		npcHandler:setMessage(MESSAGE_GREET, {
			'Whoa? You know the word! Amazing, |PLAYERNAME|! ...',
			'I should go and tell Fa\'hradin. ...',
			'Well. Why are you here anyway, |PLAYERNAME|?'
		})
	else
		npcHandler:setMessage(MESSAGE_GREET, '|PLAYERNAME|! How\'s it going these days? What brings you {here}?')
	end
	return true
end

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)

	-- To Appease the Mighty Quest
	if msgcontains(msg, "mission") and player:getStorageValue(Storage.TibiaTales.ToAppeaseTheMightyQuest) == 1 then
			npcHandler:say({
				'I should go and tell Fa\'hradin. ...',
				'I am impressed you know our address of welcome! I honour that. So tell me who sent you on a mission to our fortress?'}, cid)
			npcHandler.topic[cid] = 9
			elseif msgcontains(msg, "kazzan") and npcHandler.topic[cid] == 9 then
			npcHandler:say({
				'How dare you lie to me?!? The caliph should choose his envoys more carefully. We will not accept his peace-offering ...',
				'...but we are always looking for support in our fight against the evil Efreets. Tell me if you would like to join our fight.'}, cid)
			player:setStorageValue(Storage.TibiaTales.ToAppeaseTheMightyQuest, player:getStorageValue(Storage.TibiaTales.ToAppeaseTheMightyQuest) + 1)
	end

	if msgcontains(msg, 'passage') then
		if player:getStorageValue(Storage.DjinnWar.Faction.Marid) ~= 1 then
			npcHandler:say({
				'If you want to enter our fortress you have to become one of us and fight the Efreet. ...',
				'So, are you willing to do so?'
			}, cid)
			npcHandler.topic[cid] = 1
		else
			npcHandler:say('You already have the permission to enter Ashta\'daramai.', cid)
		end

	elseif npcHandler.topic[cid] == 1 then
		if msgcontains(msg, 'yes') then
			if player:getStorageValue(Storage.DjinnWar.Faction.Efreet) ~= 1 then
				npcHandler:say('Are you sure? You pledge loyalty to king Gabel, who is... you know. And you are willing to never ever set foot on Efreets\' territory, unless you want to kill them? Yes?', cid)
				npcHandler.topic[cid] = 2
			else
				npcHandler:say('I don\'t believe you! You better go now.', cid)
				npcHandler.topic[cid] = 0
			end

		elseif msgcontains(msg, 'no') then
			npcHandler:say('This isn\'t your war anyway, human.', cid)
			npcHandler.topic[cid] = 0
		end

	elseif npcHandler.topic[cid] == 2 then
		if msgcontains(msg, 'yes') then
			npcHandler:say({
				'Oh. Ok. Welcome then. You may pass. ...',
				'And don\'t forget to kill some Efreets, now and then.'
			}, cid)
			player:setStorageValue(Storage.DjinnWar.MaridFaction.Start, 1)
			player:setStorageValue(Storage.DjinnWar.Faction.Marid, 1)
			player:setStorageValue(Storage.DjinnWar.Faction.Greeting, 0)
			player:setStorageValue(Storage.Factions, 2)

		elseif msgcontains(msg, 'no') then
			npcHandler:say('This isn\'t your war anyway, human.', cid)
		end
		npcHandler.topic[cid] = 0
	end
	return true
end

npcHandler:setMessage(MESSAGE_FAREWELL, '<salutes>Aaaa -tention!')
npcHandler:setMessage(MESSAGE_WALKAWAY, '<salutes>Aaaa -tention!')

npcHandler:setCallback(CALLBACK_GREET, greetCallback)
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
