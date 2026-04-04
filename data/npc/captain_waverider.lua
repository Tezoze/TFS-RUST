-- Captain Waverider - Converted from XML to Lua NpcType
-- Original XML: data/npc/Captain Waverider.xml
-- Original Script: data/npc/scripts/Captain Waverider.lua

local npcName = "Captain Waverider"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a captain waverider")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 96})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


-- Special handling for peg leg access to Meriana
local pegLegKeyword = keywordHandler:addKeyword({'peg leg'}, function(cid, type, msg, matches, node)
	local player = Player(cid)
	if not player then
		npcHandler:say("Something went wrong. Please try again.", cid)
		return true
	end
	local accessValue = player:getStorageValue(Storage.TheShatteredIsles.AccessToMeriana) or -1
	if accessValue < 1 then
		npcHandler:say("Sorry, my old ears can't hear you.", cid)
		return true
	else
		npcHandler:say("Ohhhh. So... <lowers his voice> you know who sent you so I sail you to you know where. <wink> <wink> It will cost 50 gold to cover my expenses. Is it that what you wish?", cid)
		npcHandler.topic[cid] = 1 -- Set topic to allow child keywords
		return true
	end
end, {npcHandler = npcHandler})

-- Child keywords for peg leg travel
pegLegKeyword:addChildKeyword({'yes'}, function(cid)
	local player = Player(cid)
	if not player then
		npcHandler:say("Something went wrong. Please try again.", cid)
		npcHandler.topic[cid] = 0
		return true
	end
	if player:removeMoneyNpc(50) then
		npcHandler:say("And there we go!", cid)
		player:teleportTo(Position(32346, 32625, 7))
		player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
		npcHandler.topic[cid] = 0
	else
		npcHandler:say("You don't have enough money.", cid)
		npcHandler.topic[cid] = 0
	end
	return true
end, {npcHandler = npcHandler})

pegLegKeyword:addChildKeyword({'no'}, function(cid)
	npcHandler:say("I have to admit this leaves me a bit puzzled.", cid)
	npcHandler.topic[cid] = 0
	return true
end, {npcHandler = npcHandler})

-- Regular passage keyword
local passageKeyword = keywordHandler:addKeyword({'passage'}, function(cid, type, msg, matches, node)
	npcHandler:say("<sigh> I knew someone else would claim all the treasure someday. But at least it will be you and not some greedy and selfish person. For a small fee of 200 gold pieces I will sail you to your rendezvous with fate. Do we have a deal?", cid)
	npcHandler.topic[cid] = 2 -- Set topic to allow child keywords
	return true
end, {npcHandler = npcHandler})

-- Child keywords for regular passage
passageKeyword:addChildKeyword({'yes'}, function(cid)
	local player = Player(cid)
	if not player then
		npcHandler:say("Something went wrong. Please try again.", cid)
		npcHandler.topic[cid] = 0
		return true
	end
	if player:removeMoneyNpc(200) then
		npcHandler:say("And there we go!", cid)
		player:teleportTo(Position(32131, 32913, 7))
		player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
		npcHandler.topic[cid] = 0
	else
		npcHandler:say("You don't have enough money.", cid)
		npcHandler.topic[cid] = 0
	end
	return true
end, {npcHandler = npcHandler})

passageKeyword:addChildKeyword({'no'}, function(cid)
	npcHandler:say("I have to admit this leaves me a bit puzzled.", cid)
	npcHandler.topic[cid] = 0
	return true
end, {npcHandler = npcHandler})

-- Treasure Island teleport (no cost, always available)
keywordHandler:addKeyword({'map'}, function(cid, type, msg, matches, node)
	local player = Player(cid)
	if not player then
		npcHandler:say("Something went wrong. Please try again.", cid)
		return true
	end
	player:teleportTo(Position(32044, 32931, 7))
	player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
	npcHandler:say("Bon voyage!", cid)
	return true
end, {npcHandler = npcHandler})

keywordHandler:addKeyword({'treasure island'}, function(cid, type, msg, matches, node)
	local player = Player(cid)
	if not player then
		npcHandler:say("Something went wrong. Please try again.", cid)
		return true
	end
	player:teleportTo(Position(32044, 32931, 7))
	player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
	npcHandler:say("Bon voyage!", cid)
	return true
end, {npcHandler = npcHandler})

npcHandler:setMessage(MESSAGE_GREET, "Greetings, daring adventurer. If you need a {passage}, let me know.")
npcHandler:setMessage(MESSAGE_FAREWELL, "Good bye.")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Oh well.")


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
