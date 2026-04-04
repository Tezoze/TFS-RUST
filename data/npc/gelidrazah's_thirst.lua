-- Gelidrazah'S Thirst - Converted from XML to Lua NpcType
-- Original XML: data/npc/Gelidrazah'S Thirst.xml
-- Original Script: data/npc/scripts/Gelidrazahs_Thirst.lua

local npcName = "Gelidrazah'S Thirst"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a gelidrazah's thirst")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(100000)
npcType:walkRadius(2)
npcType:baseSpeed(0)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookTypeEx = 10948})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
    if not npcHandler:isFocused(cid) then
        return false
    end

    local player = Player(cid)

    if msgcontains(msg, "yes") then
        if npcHandler.topic[cid] == 0 then
            npcHandler:say({
				"There are three questions. First: What is the name of the princess who fell in love with a Thaian nobleman during the regency of pharaoh Uthemath? Second: Who is the author of the book ,The Language of the Wolves'? ...",
				"Third: Which ancient Tibian race reportedly travelled the sky in cloud ships? Can you answer these {questions}?"
			}, cid)
			npcHandler.topic[cid] = 1
		else
            npcHandler:say('I don\'t know what you are talking about.', cid)
end
    elseif msgcontains(msg, "questions") and npcHandler.topic[cid] == 1 then
		npcHandler:say("So I ask you: What is the name of the princess who fell in love with a Thaian nobleman during the regency of pharaoh Uthemath?", cid)
		npcHandler.topic[cid] = 2
    elseif msgcontains(msg, "Tahmehe") and npcHandler.topic[cid] == 2 then
        npcHandler:say("That's right. Listen to the second question: Who is the author of the book ,The Language of the Wolves'?", cid)
		npcHandler.topic[cid] = 3
    elseif msgcontains(msg, "Ishara") and npcHandler.topic[cid] == 3 then
        npcHandler:say("That's right. Listen to the third question: Which ancient Tibian race reportedly travelled the sky in cloud ships?", cid)
		npcHandler.topic[cid] = 4
	 elseif msgcontains(msg, "Svir") and npcHandler.topic[cid] == 4 then
        npcHandler:say("That is correct. You satisfactorily answered all questions. You may pass and enter Gelidrazah's lair.", cid)
		npcHandler.topic[cid] = 0
		player:setStorageValue(Storage.FirstDragon.GelidrazahAccess, 1)
    return true

end
npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
npcHandler:setMessage(MESSAGE_GREET, "Have you come to answer Gelidrazah's questions?")
npcHandler:setMessage(MESSAGE_FAREWELL, "See you, |PLAYERNAME|.")
npcHandler:setMessage(MESSAGE_WALKAWAY, "See you, |PLAYERNAME|.")
end


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
