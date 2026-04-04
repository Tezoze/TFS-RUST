-- Xorlosh - Converted from XML to Lua NpcType
-- Original XML: data/npc/Xorlosh.xml
-- Original Script: data/npc/scripts/Xorlosh.lua

local npcName = "Xorlosh"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a xorlosh")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 160, lookHead = 41, lookBody = 95, lookLegs = 75, lookFeet = 95})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)

	if msgcontains(msg, "mission") then
		if player:getStorageValue(Storage.hiddenCityOfBeregar.GoingDown) < 1 then
			npcHandler:say("Hmmmm, you could indeed help me. See this mechanism? Some son of a rotworm put WAY too much stuff on this elevator and now it's broken. I need 3 gear wheels to fix it. You think you could get them for me?", cid)
			npcHandler.topic[cid] = 1
		elseif player:getStorageValue(Storage.hiddenCityOfBeregar.GoingDown) == 1 and player:removeItem(9690, 3) then
			player:setStorageValue(Storage.hiddenCityOfBeregar.GoingDown, 2)
			npcHandler:say("HOLY MOTHER OF ALL ROTWORMS! You did it and they are of even better quality than the old ones. You should be the first one to try the elevator, just jump on it. See you my friend.", cid)
		end
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 1 then
			player:setStorageValue(Storage.hiddenCityOfBeregar.GoingDown, 1)
			player:setStorageValue(Storage.hiddenCityOfBeregar.DefaultStart, 1)
			npcHandler:say("That would be great! Maybe a blacksmith can forge you some. Come back when you got them and ask me about your mission.", cid)
			npcHandler.topic[cid] = 0
		end
	elseif msgcontains(msg, "tunnel") then
		if player:getStorageValue(Storage.hiddenCityOfBeregar.RoyalRescue) == 1 then
			npcHandler:say({
				"There should be a book in our library about tunnelling. I don't have that much time to talk to you about that. ...",
				"The book about tunnelling is in the library which is located in the north eastern wing of Beregar city."
			}, cid)
		end
	end
	return true
end

npcHandler:setMessage(MESSAGE_WALKAWAY, "See you my friend.")
npcHandler:setMessage(MESSAGE_FAREWELL, "See you my friend.")
npcHandler:setMessage(MESSAGE_GREET, "Who are you? Are you a genius in mechanics? You don't look like one.")
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
