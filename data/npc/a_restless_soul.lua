-- A Restless Soul - Converted from XML to Lua NpcType
-- Original XML: data/npc/A Restless Soul.xml
-- Original Script: data/npc/scripts/A Restless Soul.lua

local npcName = "A Restless Soul"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a a restless soul")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 48})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function greetCallback(cid)
	local player = Player(cid)
	if player:getStorageValue(Storage.TheIceIslands.Questline) < 37 then
		npcHandler:say("Uhhhh...", cid)
		return false
	end
	return true
end

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	if msgcontains(msg, "story") then
		local player = Player(cid)
		if player:getStorageValue(Storage.TheIceIslands.Questline) == 37 then
			npcHandler:say({
				"I was captured and tortured to death by the cultists here. They worship a being that they call Ghazbaran ...",
				"In his name they have claimed the mines and started to melt the ice to free an army of vile demons that have been frozen here for ages ...",
				"Their plan is to create a new demon army for their master to conquer the world. Hjaern and the other shamans must learn about it! Hurry before its too late."
			}, cid)
			player:setStorageValue(Storage.TheIceIslands.Questline, 38)
			player:setStorageValue(Storage.TheIceIslands.Mission10, 2) -- Questlog The Ice Islands Quest, Formorgar Mines 2: Ghostwhisperer
			player:setStorageValue(Storage.TheIceIslands.Mission11, 1) -- Questlog The Ice Islands Quest, Formorgar Mines 3: The Secret
		end
	end
	return true
end

npcHandler:setCallback(CALLBACK_GREET, greetCallback)
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
