-- Skjaar - Converted from XML to Lua NpcType
-- Original XML: data/npc/Skjaar.xml
-- Original Script: data/npc/scripts/Skjaar.lua

local npcName = "Skjaar"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a skjaar")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 9})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end
	local player = Player(cid)
	if msgcontains(msg, 'key') then
		npcHandler:say('I will give the key to the crypt only to the closest followers of my master. Would you like me to test you?', cid)
		npcHandler.topic[cid] = 1
	elseif msgcontains(msg, 'yes') and npcHandler.topic[cid] == 1 then
		npcHandler:say('Before we start I must ask you for a small donation of 1000 gold coins. Are you willing to pay 1000 gold coins for the test?', cid)
		npcHandler.topic[cid] = 2
	elseif msgcontains(msg, 'yes') and npcHandler.topic[cid] == 2 then
		if player:removeMoneyNpc(1000) then
			npcHandler:say('All right then. Here comes the first question. What was the name of Dago\'s favourite pet?', cid)
			npcHandler.topic[cid] = 3
		else
			npcHandler:say('You don\'t have enough money', cid)
			npcHandler.topic[cid] = 0
		end
	elseif msgcontains(msg, 'redips') and npcHandler.topic[cid] == 3 then
		npcHandler:say('Perhaps you knew him after all. Tell me - how many fingers did he have when he died?', cid)
		npcHandler.topic[cid] = 4
	elseif msgcontains(msg, '7') and npcHandler.topic[cid] == 4 then
		npcHandler:say('Also true. But can you also tell me the colour of the deamons in which master specialized?', cid)
		npcHandler.topic[cid] = 5
	elseif msgcontains(msg, 'black') and npcHandler.topic[cid] == 5 then
		npcHandler:say('It seems you are worthy after all. Do you want the key to the crypt?', cid)
		npcHandler.topic[cid] = 6
	elseif msgcontains(msg, 'yes') and npcHandler.topic[cid] == 6 then
		npcHandler:say('Here you are', cid)
		local key = player:addItem(2089, 1)
		if key then
			key:setActionId(3142)
		end
	end
	return true
end

npcHandler:setMessage(MESSAGE_GREET, "Another creature who believes thinks physical strength is more important than wisdom! Why are you disturbing me?")
npcHandler:setMessage(MESSAGE_FAREWELL, "Farewell, |PLAYERNAME|!")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Run away, unworthy |PLAYERNAME|!")

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
