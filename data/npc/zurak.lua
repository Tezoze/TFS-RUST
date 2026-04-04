-- zurak - Converted from XML to Lua NpcType
-- Original XML: data/npc/zurak.xml
-- Original Script: data/npc/scripts/Zurak.lua

local npcName = "Zurak"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a zurak")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(0)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 114})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	elseif msgcontains(msg, "trip") or msgcontains(msg, "passage") then
		--if Player(cid):getStorageValue(Storage.TheNewFrontier.Questline) >= 24 then
			npcHandler:say("You want trip to Izzle of Zztrife?", cid)
			npcHandler.topic[cid] = 1
			--else
			--npcHandler:say("You need The New Frontier Quest to travel.", cid)
		--end
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 1 then
			npcHandler:say("It'zz your doom you travel to.", cid)
			local player, destination = Player(cid), Position(33102, 31056, 7)
			player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
			player:teleportTo(destination)
			destination:sendMagicEffect(CONST_ME_TELEPORT)
			npcHandler.topic[cid] = 0
		end
	elseif msgcontains(msg, "no") then
		if npcHandler.topic[cid] == 1 then
			npcHandler:say("Zzoftzzkinzz zzo full of fear.", cid)
			npcHandler.topic[cid] = 0
		end
	elseif msgcontains(msg, 'hurry') or msgcontains(msg, 'job')  then
		npcHandler:say('Me zzimple ferryman. I arrange {trip} to Izzle of Zztrife.', cid)
		npcHandler.topic[cid] = 0
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
