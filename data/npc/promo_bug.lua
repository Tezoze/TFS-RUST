-- Promo Bug - Converted from XML to Lua NpcType
-- Original XML: data/npc/Promo Bug.xml
-- Original Script: data/npc/scripts/promo bug.lua

local npcName = "Promo Bug"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a promo bug")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 153, lookHead = 39, lookBody = 39, lookLegs = 39, lookFeet = 76})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end


	local player = Player(cid)

	if msgcontains(msg, "promot") then
	local vocation = player:getVocation()
    local promotion = vocation:getPromotion()
    if player:isPremium() then
        local value = player:getStorageValue(Storage.Promotion)
        if not promotion and value ~= 1 then
		npcHandler:say({"You want be promoted? 1"}, cid)
		npcHandler.topic[cid] = 1
    elseif value == 1 then
		npcHandler:say({"You want be promoted? 2"}, cid)
		npcHandler.topic[cid] = 1
		end
    elseif not promotion then
        player:setVocation(vocation:getDemotion())
    end

	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 1 then
		local vocation = player:getVocation()
		local promotion = vocation:getPromotion()
			player:setVocation(promotion)
			--player:setVocation(vocation:getDemotion())
			npcHandler:say("Promotion Done.", cid)
			npcHandler.topic[cid] = 0
		else
			npcHandler:say("Zzz...", cid)
		end

		-- YES AQUI

	elseif msgcontains(msg, "no") then
		if npcHandler.topic[cid] == 1 then
			npcHandler:say("Then no.", cid)
			npcHandler.topic[cid] = 0
		end
	end
		-- YES AQUI

	return true

end
npcHandler:setMessage(MESSAGE_WALKAWAY, "Bye, bye.")
npcHandler:setMessage(MESSAGE_FAREWELL, "Bye, bye.")
npcHandler:setMessage(MESSAGE_GREET, "Hiho, hiho |PLAYERNAME|.")
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
