-- Timothy - Converted from XML to Lua NpcType
-- Original XML: data/npc/Timothy.xml
-- Original Script: data/npc/scripts/Timothy.lua

local npcName = "Timothy"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a timothy")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 128, lookHead = 58, lookBody = 61, lookLegs = 25, lookFeet = 57})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if(not npcHandler:isFocused(cid)) then
		return false
	end
	local player = Player(cid)
	if(msgcontains(msg, "blood crystal")) then
		npcHandler:say("Oh yes, I heard about a gem like that. Nowadays they seem incredibly hard to acquire. The explorer's society doesn't own one, else I'd help you.", cid)
	elseif(msgcontains(msg, "research notes")) then
		if player:getStorageValue(Storage.TheWayToYalahar.QuestLine) == 1 then
			npcHandler:say({
				"Oh, you are the contact person of the academy? Here are the notes that contain everything I have found out so far. ...",
				"This city is absolutely fascinating, I tell you! If there hadn't been all this trouble and chaos in the past, this city would certainly be the greatest centre of knowledge in the world. ...",
				"Oh, by the way, speaking about all the trouble here reminds me of Palimuth, a friend of mine. He is a native who was quite helpful in gathering all these information. ...",
				"I'd like to pay him back for his kindness by sending him some experienced helper that assists him in his effort to restore some order in this city. Maybe you are interested in this job?"
			}, cid)
			npcHandler.topic[cid] = 1
		end
	elseif(msgcontains(msg, "yes")) then
		if(npcHandler.topic[cid] == 1) then
			player:setStorageValue(Storage.TheWayToYalahar.QuestLine, 2)
			npcHandler:say("Excellent! You will find Palimuth near the entrance of the city centre. Just ask him if you can assist him in a few missions.", cid)
			player:addItem(10090, 1)
			npcHandler.topic[cid] = 0
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
