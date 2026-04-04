-- Angelina - Converted from XML to Lua NpcType
-- Original XML: data/npc/Angelina.xml
-- Original Script: data/npc/scripts/Angelina.lua

local npcName = "Angelina"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a angelina")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 136, lookHead = 57, lookBody = 117, lookLegs = 118, lookFeet = 114})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function greetCallback(cid)
	local player = Player(cid)
	local storageValue = player:getStorageValue(Storage.OutfitQuest.MageSummoner.AddonWand) or -1
	if storageValue < 1 then
		npcHandler:setMessage(MESSAGE_GREET, "The gods must be praised that I am finally saved. I do not have many worldly possessions, but please accept a small reward, do you?")
	elseif	storageValue >= 1 then
		npcHandler:setMessage(MESSAGE_GREET, "Thanks for saving my life! Should I teleport you out of the Dark Cathedral?")
	end
	return true
end

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	if msgcontains(msg, "Yes") then
		local storageValue = player:getStorageValue(Storage.OutfitQuest.MageSummoner.AddonWand) or -1
		if storageValue < 1 then
			npcHandler:say("I will tell you a small secret now. My friend Lynda in Thais can create a blessed wand. Greet her from me, maybe she will aid you.", cid)
			player:setStorageValue(Storage.OutfitQuest.MageSummoner.AddonWand, 1)
			player:setStorageValue(Storage.OutfitQuest.DefaultStart, 1) --this for default start of Outfit and Addon Quests
		elseif storageValue >= 1 then
			player:teleportTo(Position(32659, 32340, 7))
			player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
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
