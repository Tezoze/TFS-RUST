-- Gnome Trooper - Converted from XML to Lua NpcType
-- Original XML: data/npc/Gnome Trooper.xml
-- Original Script: data/npc/scripts/Gnome Trooper.lua

local npcName = "Gnome Trooper"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a gnome trooper")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 493, lookHead = 59, lookBody = 20, lookLegs = 39, lookFeet = 95})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)
local response = {
	[0] = "It's a pipe! What can be more relaxing for a gnome than to smoke his pipe after a day of duty at the front. At least it's a chance to do something really dangerous after all!",
	[1] = "Ah, a letter from home! Oh - I had no idea she felt that way! This is most interesting!",
	[2] = "It's a model of the gnomebase Alpha! For self-assembly! With toothpicks...! Yeeaah...! I guess.",
	[3] = "A medal of honour! At last they saw my true worth!"
}

if not DELIVERED_PARCELS then
	DELIVERED_PARCELS = {}
end


function greetCallback(cid)
	local player = Player(cid)
	if isInArray({-1, 4}, player:getStorageValue(SPIKE_LOWER_PARCEL_MAIN)) then
		return false
	end
	if isInArray(DELIVERED_PARCELS[player:getGuid()], Creature(getNpcCid()):getId()) then
		return false
	end
	return true
end

function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	local status = player:getStorageValue(SPIKE_LOWER_PARCEL_MAIN)

	if not DELIVERED_PARCELS[player:getGuid()] then
		DELIVERED_PARCELS[player:getGuid()] = {}
	end

	if msgcontains(msg, 'something') and not isInArray({-1, 4}, status) then
		if isInArray(DELIVERED_PARCELS[player:getGuid()], Creature(getNpcCid()):getId()) then
			return true
		end

		if not player:removeItem(21569, 1) then
			npcHandler:say("But you don't have it...", cid)
			return npcHandler:releaseFocus(cid)
		end

		npcHandler:say(response[player:getStorageValue(SPIKE_LOWER_PARCEL_MAIN)], cid)
		player:setStorageValue(SPIKE_LOWER_PARCEL_MAIN, status + 1)
		table.insert(DELIVERED_PARCELS[player:getGuid()], Creature(getNpcCid()):getId())
		npcHandler:releaseFocus(cid)
	return true

end
npcHandler:setCallback(CALLBACK_GREET, greetCallback)
npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
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
