-- LuciusLight - Converted from XML to Lua NpcType
-- Original XML: data/npc/LuciusLight.xml
-- Original Script: data/npc/scripts/Luciuslight.lua

local npcId = "LuciusLight"  -- Unique ID for spawn system
local npcDisplayName = "Lucius"  -- Name shown in-game
local npcType = Game.createNpcType(npcId)

-- NPC Properties (from XML)
npcType:name(npcDisplayName)
npcType:nameDescription("a lucius")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 325, lookHead = 76, lookBody = 79, lookLegs = 117, lookFeet = 114, lookAddons = 3})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)




local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end
	-- 45250  Quest iniciado
	-- 45251  Antorchas
	-- 45252  Puerta
	-- 45253  Cofre
	
	local player = Player(cid)
	if(msgcontains(msg, "mission")) then
		if(player:getStorageValue(45250) == -1) then			
			player:setStorageValue(45250, 1)
			player:setStorageValue(45251, 0)
			doPlayerAddItem(player,9956,1)
			npcHandler:say({
					'This magical torch will be used to light Lightbringer\'s Basin all around the world, which can be done in any order...',
					'To complete the mission, use the Magical torch in 10 Lightbringer\'s Basin...',
					'If the torch finished first.. I can give you another one for $5,000 gold coins ... you\'ll have to start again...',
					'around the world there are at least 15 Lightbringer\'s Basin.. Good luck'
				}, cid)			
		end
		
		if(player:getStorageValue(45251) >= 10  and player:getStorageValue(45252) == -1) then
			player:setStorageValue(45252, 1)		
			npcHandler:say({
					'Congratulations you have accomplished the mission!! ...',
					'Take your reward'
				}, cid)	
		end
	elseif (msgcontains(msg, "torch") and player:getStorageValue(45252) == -1) then
		if player:removeMoneyNpc(5000) then
			player:setStorageValue(45251, 0)
			doPlayerAddItem(player,9956,1)
			npcHandler:say({
					'This magical torch will be used to light Lightbringer\'s Basin all around the world, which can be done in any order...',
					'To complete the mission, light 10 Basin..',
					'If the torch finished first.. I can give you another one for $5,000 gold coins ... you\'ll have to start again...',
					'around the world there are at least 15 Lightbringer\'s Basin.. Good luck'
				}, cid)	
		else
			npcHandler:say({
					'I can give you another one for $5,000 gold coins ... you\'ll have to start again...'
				}, cid)		
		end
		
	else
		if player:removeMoneyNpc(5000) then
			doPlayerAddItem(player,9956,1)
			npcHandler:say({
					'You compleat the mission.. but if you need more take it...'
				}, cid)	
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
