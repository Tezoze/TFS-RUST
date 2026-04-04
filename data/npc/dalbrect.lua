-- Dalbrect - Converted from XML to Lua NpcType
-- Original XML: data/npc/Dalbrect.xml
-- Original Script: data/npc/scripts/Dalbrect.lua

local npcName = "Dalbrect"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a dalbrect")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 129, lookHead = 76, lookBody = 97, lookLegs = 67, lookFeet = 76})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)

	if msgcontains(msg, 'brooch') then
		if player:getStorageValue(Storage.WhiteRavenMonasteryQuest.Passage) == 1 then
			npcHandler:say('You have recovered my brooch! I shall forever be in your debt, my friend!', cid)
			return true
		end

		npcHandler:say('What? You want me to examine a brooch?', cid)
		npcHandler.topic[cid] = 1
	elseif msgcontains(msg, 'yes') then
		if npcHandler.topic[cid] == 1 then
			if player:getItemCount(2318) == 0 then
				npcHandler:say('What are you talking about? I am too poor to be interested in jewelry.', cid)
				npcHandler.topic[cid] = 0
				return true
			end

			npcHandler:say('Can it be? I recognise my family\'s arms! You have found a treasure indeed! I am poor and all I can offer you is my friendship, but ... please ... give that brooch to me?', cid)
			npcHandler.topic[cid] = 2
		elseif npcHandler.topic[cid] == 2 then
			npcHandler.topic[cid] = 0
			if not player:removeItem(2318, 1) then
				npcHandler:say('I should have known better than to ask for an act of kindness in this cruel, selfish, world!', cid)
				return true
			end

			npcHandler:say('Thank you! I shall consider you my friend from now on! Just let me know if you {need} something!', cid)
			player:setStorageValue(Storage.WhiteRavenMonasteryQuest.Passage, 1)
			player:setStorageValue(Storage.WhiteRavenMonasteryQuest.QuestLog, 1) -- Quest log
		end
	elseif msgcontains(msg, 'no') then
		if npcHandler.topic[cid] == 1 then
			npcHandler:say('Then stop being a fool. I am poor and I have to work the whole day through!', cid)
		elseif npcHandler.topic[cid] == 2 then
			npcHandler:say('I should have known better than to ask for an act of kindness in this cruel, selfish, world!', cid)
		end
		npcHandler.topic[cid] = 0
	end
	return true
end

-- Travel to Isle of Kings
local travelKeyword = keywordHandler:addKeyword({'passage'}, function(cid, type, msg, matches, node)
    local player = Player(cid)
    if not player then
        npcHandler:say("Something went wrong. Please try again.", cid)
        return true
    end
    local accessValue = player:getStorageValue(Storage.WhiteRavenMonasteryQuest.Passage) or -1
    if accessValue < 1 then
        npcHandler:say('I have only sailed to the isle of the kings once or twice. I dare not anger the monks by bringing travellers there without their permission.', cid)
        return true
    else
        npcHandler:say('Do you seek a passage to the isle of the kings for 10 gold coins?', cid)
        npcHandler.topic[cid] = 3 -- Set topic to allow child keywords
        return true
    end
end, {npcHandler = npcHandler})

-- Child keywords for travel
travelKeyword:addChildKeyword({'yes'}, function(cid)
    local player = Player(cid)
    if not player then
        npcHandler:say("Something went wrong. Please try again.", cid)
        npcHandler.topic[cid] = 0
        return true
    end
    if not player:removeMoney(10) then
        npcHandler:say("You don't have enough money.", cid)
        npcHandler.topic[cid] = 0
        return true
    end
    player:teleportTo(Position(32190, 31957, 6))
    Position(32190, 31957, 6):sendMagicEffect(CONST_ME_TELEPORT)
    npcHandler:say("Have a nice trip!", cid)
    npcHandler.topic[cid] = 0
    return true
end, {npcHandler = npcHandler, premium = false, cost = 10, destination = Position(32190, 31957, 6)})
travelKeyword:addChildKeyword({'no'}, StdModule.say, {npcHandler = npcHandler, text = 'Well, I\'ll be here if you change your mind.', reset = true})

keywordHandler:addKeyword({'name'}, StdModule.say, {npcHandler = npcHandler, text = "My name is Dalbrect Windtrouser, of the once proud Windtrouser family."})
keywordHandler:addKeyword({'hut'}, StdModule.say, {npcHandler = npcHandler, text = "I am merely a humble fisher now that nothing is left of my noble {legacy}."})
keywordHandler:addKeyword({'legacy'}, StdModule.say, {npcHandler = npcHandler, text = "Once my family was once noble and wealthy, but {fate} turned against us and threw us into poverty."})
keywordHandler:addKeyword({'poverty'}, StdModule.say, {npcHandler = npcHandler, text = "When Carlin tried to colonize the region now known as the ghostlands, my ancestors put their fortune in that {project}."})
keywordHandler:addKeyword({'fate'}, StdModule.say, {npcHandler = npcHandler, text = "When Carlin tried to colonize the region now known as the ghostlands, my ancestors put their fortune in that {project}."})
keywordHandler:addKeyword({'ghostlands'}, StdModule.say, {npcHandler = npcHandler, text = "Our family fortune was lost when the colonization of those cursed lands failed. Now nothing is left of our fame or our fortune. If I only had something as a reminder of those better times. <sigh>"})
keywordHandler:addKeyword({'project'}, StdModule.say, {npcHandler = npcHandler, text = "Our family fortune was lost when the colonization of those cursed lands failed. Now nothing is left of our fame or our fortune. If I only had something as a reminder of those better times. <sigh>"})
keywordHandler:addKeyword({'carlin'}, StdModule.say, {npcHandler = npcHandler, text = "To think my family used to belong to the local nobility! And now those arrogant women are in charge!"})
keywordHandler:addKeyword({'need'}, StdModule.say, {npcHandler = npcHandler, text = "There is little I can offer you but a trip with my boat. Are you looking for a {passage} to the isle of kings perhaps?"})
keywordHandler:addKeyword({'ship'}, StdModule.say, {npcHandler = npcHandler, text = "My ship is my only pride and joy."})

npcHandler:setMessage(MESSAGE_GREET, "Be greeted, traveller |PLAYERNAME|.")
npcHandler:setMessage(MESSAGE_FAREWELL, "Good bye. You are welcome.")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Good bye.")

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
