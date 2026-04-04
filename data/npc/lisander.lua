-- Lisander - Converted from XML to Lua NpcType
-- Original XML: data/npc/Lisander.xml
-- Original Script: data/npc/scripts/Lisander.lua

local npcName = "Lisander"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a lisander")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 146, lookHead = 94, lookBody = 100, lookLegs = 117, lookFeet = 115})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	
	-- Blood Brothers Quest - Garlic Cookie test
	if msgcontains(msg, "garlic cookie") or msgcontains(msg, "cookie") then
		if player:getStorageValue(Storage.BloodBrothers.Mission02) == 1 then
		if player:getItemCount(9116) > 0 then -- Garlic Cookie item ID
			player:removeItem(9116, 1)
				local currentCount = player:getStorageValue(Storage.BloodBrothers.GarlicCookieCount)
				if currentCount == -1 then currentCount = 0 end
				player:setStorageValue(Storage.BloodBrothers.GarlicCookieCount, currentCount + 1)
				player:setStorageValue(Storage.BloodBrothers.LisanderSuspect, 1)
				npcHandler:say("Oh, a cookie! How wonderful! *happily eats the cookie* It tastes quite interesting with that garlic flavor!", cid)
			else
				npcHandler:say("A cookie would be nice, but I don't see one with you.", cid)
			end
		else
			npcHandler:say("Thank you for the offer, but I'm not hungry at the moment.", cid)
		end
	elseif msgcontains(msg, "blood crystal") then
		npcHandler:say("No, that's wine I'm drinking.", cid)
		return true
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
