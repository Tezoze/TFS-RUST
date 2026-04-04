-- Rata'Mari - Converted from XML to Lua NpcType
-- Original XML: data/npc/Rata'Mari.xml
-- Original Script: data/npc/scripts/Rata_mari.lua

local npcName = "Rata'Mari"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a rata'mari")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 21})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function greetCallback(cid)
	if Player(cid):getStorageValue(Storage.DjinnWar.MaridFaction.Mission02) == -1 then
		return false
	end

	return true
end

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)

	if msgcontains(msg, 'spy report') then
		local reportProgress = player:getStorageValue(Storage.DjinnWar.MaridFaction.RataMari)
		if reportProgress < 1 then
			npcHandler:say({
				'You have come for the report? Great! I have been working hard on it during the last months. And nobody came to pick it up. I thought everybody had forgotten about me! ...',
				'Do you have any idea how difficult it is to hold a pen when you have claws instead of hands? ...',
				'But - you know - now I have worked so hard on this report I somehow don\'t want to part with it. At least not without some decent payment. ...',
				'All right - listen - I know Fa\'hradin would not approve of this, but I can\'t help it. I need some cheese! I need it now! ...',
				'And I will not give the report to you until you get me some! Meep!'
			}, cid)
			player:setStorageValue(Storage.DjinnWar.MaridFaction.RataMari, 1)

		elseif reportProgress == 1 then
			npcHandler:say('Ok, have you brought me the cheese, I\'ve asked for?', cid)
			npcHandler.topic[cid] = 1
		else
			npcHandler:say('I already gave you the report. I\'m not going to write another one!', cid)
		end

	elseif npcHandler.topic[cid] == 1 then
		if msgcontains(msg, 'yes') then
			if not player:removeItem(2696, 1) then
				npcHandler:say('No cheese - no report.', cid)
				return true
			end

			player:setStorageValue(Storage.DjinnWar.MaridFaction.RataMari, 2)
			player:addItem(2345, 1)
			npcHandler:say('Meep! Meep! Great! Here is the spyreport for you!', cid)
		else
			npcHandler:say('No cheese - no report.', cid)
		end
		npcHandler.topic[cid] = 0
	end
	return true
end

keywordHandler:addKeyword({'rat'}, StdModule.say, {npcHandler = npcHandler, text = 'Your power of observation is stunning. Yes, I\'m a rat.'})

npcHandler:setMessage(MESSAGE_GREET, "Meep? I mean - hello! Sorry, |PLAYERNAME|... Being a {rat} has kind of grown on me.")
npcHandler:setMessage(MESSAGE_FAREWELL, "Meep!")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Meep!")

npcHandler:setCallback(CALLBACK_GREET, greetCallback)
npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)

local focusModule = FocusModule:new()
focusModule:addGreetMessage('piedpiper')
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
