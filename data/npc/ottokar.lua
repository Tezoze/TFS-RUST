-- Ottokar - Converted from XML to Lua NpcType
-- Original XML: data/npc/Ottokar.xml
-- Original Script: data/npc/scripts/Ottokar.lua

local npcName = "Ottokar"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a ottokar")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 153, lookHead = 132, lookBody = 121, lookLegs = 120, lookFeet = 114, lookAddons = 3})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end
	local player = Player(cid)
	if msgcontains(msg, 'belongings of deceasead') or msgcontains(msg, 'medicine') then
		if player:getItemCount(13506) > 0 then
			npcHandler:say('Did you bring me the medicine pouch?', cid)
			npcHandler.topic[cid] = 1
		else
			npcHandler:say('I need a {medicine pouch}, to give you the {belongings of deceased}. Come back when you have them.', cid)
			npcHandler.topic[cid] = 0
		end
	elseif msgcontains(msg, 'yes') and npcHandler.topic[cid] == 1 then
		if player:removeItem(13506, 1) then
			player:addItem(13670, 1)
			player:addAchievementProgress('Doctor! Doctor!', 100)
			npcHandler:say('Here you are', cid)
		else
			npcHandler:say('You do not have the required items.', cid)
		end
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
