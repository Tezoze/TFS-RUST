-- Tarakinner - Converted from XML to Lua NpcType
-- Original XML: data/npc/Tarakinner.xml
-- Original Script: data/npc/scripts/TarakInner.lua

local npcName = "Tarak"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a tarak")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 153, lookHead = 115, lookBody = 31, lookLegs = 66, lookFeet = 97})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end
	if msgcontains(msg, "monument tower") or msgcontains(msg, "passage") or msgcontains(msg, "trip") then
		npcHandler:say("Do you want to travel to the {monument tower} for a 50 gold fee?", cid)
		npcHandler.topic[cid] = 1
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 1 then
			local player = Player(cid)
			if player:getMoney() + player:getBankBalance() >= 50 then
				player:removeMoneyNpc(50)
				player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
				player:teleportTo(Position(32940, 31182, 7), false)
				player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
				npcHandler.topic[cid] = 0

			elseif player:getBankBalance() >= 50 then
				getBankMoney(cid, 50)
				player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
				player:teleportTo(Position(32940, 31182, 7), false)
				player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
				npcHandler.topic[cid] = 0
			else
				npcHandler:say("You don't have enought money.", cid)
				npcHandler.topic[cid] = 0
			end
		end
	end
	return true
end

npcHandler:setMessage(MESSAGE_GREET, "Can I interest you in a trip to the {monument tower}?")
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
