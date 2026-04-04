-- Dermot - Converted from XML to Lua NpcType
-- Original XML: data/npc/Dermot.xml
-- Original Script: data/npc/scripts/Dermot.lua

local npcName = "Dermot"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a dermot")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 129, lookHead = 57, lookBody = 49, lookLegs = 19, lookFeet = 95})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end
	local player = Player(cid)
	if msgcontains(msg, "present") then
		if player:getStorageValue(Storage.postman.Mission05) == 2 then
			npcHandler:say("You have a present for me?? Realy?", cid)
			npcHandler.topic[cid] = 1
		end
	elseif msgcontains(msg, "key") then
		npcHandler:say("Do you want to buy the dungeon key for 2000 gold?", cid)
		npcHandler.topic[cid] = 2
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 1 then
			if player:removeItem(2331, 1) then
				npcHandler:say("Thank you very much!", cid)
				player:setStorageValue(Storage.postman.Mission05, 3)
				npcHandler.topic[cid] = 0
			end
		elseif npcHandler.topic[cid] == 2 then
			if player:removeMoneyNpc(2000) then
				npcHandler:say("Here it is.", cid)
				local key = player:addItem(2087, 1)
				if key then
					key:setActionId(3940)
				end
			else
				npcHandler:say("You don't have enough money.", cid)
			end
			npcHandler.topic[cid] = 0
		end
	end
	return true
end

keywordHandler:addKeyword({'job'}, StdModule.say, {npcHandler = npcHandler, text = "I am the magistrate of this isle."})
keywordHandler:addKeyword({'magistrate'}, StdModule.say, {npcHandler = npcHandler, text = "Thats me."})
keywordHandler:addKeyword({'name'}, StdModule.say, {npcHandler = npcHandler, text = "I am Dermot, the magistrate of this isle."})
keywordHandler:addKeyword({'time'}, StdModule.say, {npcHandler = npcHandler, text = "Time is not important on Fibula."})
keywordHandler:addKeyword({'fibula'}, StdModule.say, {npcHandler = npcHandler, text = "You are at Fibula. This isle is not very dangerous. Just the wolves bother outside the village."})
keywordHandler:addKeyword({'dungeon'}, StdModule.say, {npcHandler = npcHandler, text = "Oh, my god. In the dungeon of Fibula are a lot of monsters. That's why we have sealed it with a solid door."})
keywordHandler:addKeyword({'monsters'}, StdModule.say, {npcHandler = npcHandler, text = "Oh, my god. In the dungeon of Fibula are a lot of monsters. That's why we have sealed it with a solid door."})

npcHandler:setMessage(MESSAGE_GREET, "Hello, traveller |PLAYERNAME|. How can I help you?")
npcHandler:setMessage(MESSAGE_FAREWELL, "See you again.")
npcHandler:setMessage(MESSAGE_WALKAWAY, "See you again.")

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
