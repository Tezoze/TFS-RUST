-- Zarifan - Converted from XML to Lua NpcType
-- Original XML: data/npc/Zarifan.xml
-- Original Script: data/npc/scripts/Zarifan.lua

local npcName = "Zarifan"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a zarifan")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 560})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local voices = {
	{ text = '<sigh> lost... word...' },
	{ text = '<sigh> ohhhh.... memories...' },
	{ text = 'The secrets... too many... sleep...' },
	{ text = 'Loneliness...' }
}

local function creatureSayCallback(cid, type, msg)
if not npcHandler:isFocused(cid) then
		return false
	end

local player = Player(cid)
	if msgcontains(msg, "magic") and player:getStorageValue(12902) < 1 then
	npcHandler:say("...Tell me...the first... magic word.", cid)
	player:setStorageValue(12902, 1)
	else npcHandler:say("...continue with your mission...", cid)
	end

	end
keywordHandler:addKeyword({'mission'}, StdModule.say, {npcHandler = npcHandler, text = '..what about {magic}..'})
keywordHandler:addKeyword({'friendship'}, StdModule.say, {npcHandler = npcHandler, text = 'Yes... YES... friendship... now... second word?'})
keywordHandler:addKeyword({'lives'}, StdModule.say, {npcHandler = npcHandler, text = 'Yes... YES... friendship... lives... now third word?'})
keywordHandler:addKeyword({'forever'}, StdModule.say, {npcHandler = npcHandler, text = 'Yes... YES... friendship... lives... FOREVER ... And say hello... to... my old friend... Omrabas. '})


npcHandler:addModule(VoiceModule:new(voices))
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
