-- Rock Steady - Converted from XML to Lua NpcType
-- Original XML: data/npc/Rock Steady.xml
-- Original Script: data/npc/scripts/Rock Steady.lua

local npcName = "Rock Steady"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a rock steady")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(0)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookTypeEx = 14898})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)

	if msgcontains(msg, "addon") or msgcontains(msg, "help") then
		if player:getStorageValue(72326) < 1 then
			selfSay("If you want anything, you should talk to Old Rock Boy over there. I do {collect} stuff, though. So just ask if you're interested in helping me.", cid)
			player:setStorageValue(72326, 1)
			npcHandler.topic[cid] = 0
		end
	elseif msgcontains(msg, "collect") then
		if player:getStorageValue(72326) == 1 then
			selfSay("I collect everything that reflects light in strange ways. However, I am bored by my collection. And there wasn't anything new to add for years. ...", cid)
			selfSay("I like pearls for example - but I have already enough. I also like shells - but I can't even count how many I already own. ...", cid)
			npcHandler:say("If you find anything of REAL VALUE - bring it to me. I will reward you well. You don't already have something for me by chance?", cid)
			npcHandler.topic[cid] = 1
		elseif player:getStorageValue(72326) == 2 then
			selfSay("Have you got anything for me today?", cid)
			npcHandler.topic[cid] = 2
		elseif player:getStorageValue(72326) == 3 then
			selfSay("Have you got anything for me today?", cid)
			npcHandler.topic[cid] = 3
		elseif player:getStorageValue(72326) == 4 and player:removeItem(15434, 1) then
			selfSay("Have you got anything... what? You want what? A reward? HAHAHAHAAAA!! ...", cid)
			selfSay("No I'm just teasing you. I'm really happy about my collection now. ...", cid)
			npcHandler:say("Well, I found some kind of weapon a long time ago. I believe it may be especially helpful underwater as it is from the deep folk. In any case it is of more use for you than it would be for me.", cid)
			player:addOutfitAddon(464, 1)
			player:addOutfitAddon(463, 1)
			player:setStorageValue(72326, 5)
			npcHandler.topic[cid] = 0
		end
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 1 then
			selfSay("Great! Let me see. Amazing! I will take this, thank you!", cid)
			player:setStorageValue(72326, 2)
			npcHandler.topic[cid] = 0
		elseif npcHandler.topic[cid] == 2 and player:removeItem(15435, 1) then
			selfSay("Great! Let me see. Amazing! I will take this, thank you!", cid)
			player:setStorageValue(72326, 3)
			npcHandler.topic[cid] = 0
		elseif npcHandler.topic[cid] == 3 and player:removeItem(15436, 1) then
			selfSay("Great! Let me see. Amazing! I will take this, thank you!", cid)
			player:setStorageValue(72326, 4)
			npcHandler.topic[cid] = 0
			else selfSay("You dont have the required items!", cid)
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
