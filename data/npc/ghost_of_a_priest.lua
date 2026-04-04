-- Ghost Of A Priest - Converted from XML to Lua NpcType
-- Original XML: data/npc/Ghost Of A Priest.xml
-- Original Script: data/npc/scripts/Ghost Of A Priest.lua

local npcName = "Ghost Of A Priest"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a ghost of a priest")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 355})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Per-NPC handler storage to prevent state sharing between multiple instances
local npcHandlers = {}

local function getHandlers(npc)
	local npcId = npc:getId()
	if not npcHandlers[npcId] then
		npcHandlers[npcId] = {
			keywordHandler = KeywordHandler:new(),
			npcHandler = nil
		}
		npcHandlers[npcId].npcHandler = NpcHandler:new(npcHandlers[npcId].keywordHandler)
		npcHandlers[npcId].npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
		npcHandlers[npcId].npcHandler:addModule(FocusModule:new())
	end
	return npcHandlers[npcId]
end


local function creatureSayCallback(cid, type, msg)
	local npc = getCurrentNpc()
	if not npc then
		return false
	end
	local handlers = getHandlers(npc)
	local npcHandler = handlers.npcHandler
	
	if not npcHandler:isFocused(cid) then
		return false
	end
	local player = Player(cid)
	if msgcontains(msg, "mission") then
		if player:getStorageValue(Storage.WrathoftheEmperor.Questline) == 10 then
			if player:getPosition().z == 12 and player:getStorageValue(Storage.WrathoftheEmperor.GhostOfAPriest01) < 1 and npcHandler.topic[cid] ~= 1 then
				npcHandler:say({
					"Although we are willing to hand this item to you, there is something you have to understand: There is no such thing as 'the' sceptre. ...",
					"Those sceptres are created for special purposes each time anew. Therefore you will have to create one on your own. It will be your {mission} to find us three keepers and to get the three parts of the holy sceptre. ...",
					"Then go to the holy altar and create a new one."
				}, cid)
				npcHandler.topic[cid] = 1
			elseif npcHandler.topic[cid] == 1 then
				npcHandler:say({
					"Even though we are spirits, we can't create anything out of thin air. You will have to donate some precious metal which we can drain for energy and substance. ...",
					"The equivalent of 5000 gold will do. Are you willing to make such a donation?"
				}, cid)
				npcHandler.topic[cid] = 2
			elseif player:getPosition().z == 13 and player:getStorageValue(Storage.WrathoftheEmperor.GhostOfAPriest02) < 1 then
				npcHandler:say({
					"Even though we are spirits, we can't create anything out of thin air. You will have to donate some precious metal which we can drain for energy and substance. ...",
					"The equivalent of 5000 gold will do. Are you willing to make such a donation?"
				}, cid)
				npcHandler.topic[cid] = 3
			elseif player:getPosition().z == 14 and player:getStorageValue(Storage.WrathoftheEmperor.GhostOfAPriest03) < 1 then
				npcHandler:say({
					"Even though we are spirits, we can't create anything out of thin air. You will have to donate some precious metal which we can drain for energy and substance. ...",
					"The equivalent of 5000 gold will do. Are you willing to make such a donation?"
				}, cid)
				npcHandler.topic[cid] = 4
			end
		end
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 2 then
			if player:getMoney() + player:getBankBalance() >= 5000 then
				player:setStorageValue(Storage.WrathoftheEmperor.GhostOfAPriest01, 1)
				player:removeMoneyNpc(5000)
				player:addItem(12324, 1)
				npcHandler:say("So be it! Here is my part of the sceptre. Combine it with the other parts on the altar of the Great Snake in the depths of this temple.", cid)
				npcHandler.topic[cid] = 0
			end
		elseif npcHandler.topic[cid] == 3 then
			if player:getMoney() + player:getBankBalance() >= 5000 then
				player:setStorageValue(Storage.WrathoftheEmperor.GhostOfAPriest02, 1)
				player:removeMoneyNpc(5000)
				player:addItem(12325, 1)
				npcHandler:say("So be it! Here is my part of the sceptre. Combine it with the other parts on the altar of the Great Snake in the depths of this temple.", cid)
				npcHandler.topic[cid] = 0
			end
		elseif npcHandler.topic[cid] == 4 then
			if player:getMoney() + player:getBankBalance() >= 5000 then
				player:setStorageValue(Storage.WrathoftheEmperor.GhostOfAPriest03, 1)
				player:removeMoneyNpc(5000)
				player:addItem(12326, 1)
				npcHandler:say("So be it! Here is my part of the sceptre. Combine it with the other parts on the altar of the Great Snake in the depths of this temple.", cid)
				npcHandler.topic[cid] = 0
			end
		end
	elseif msgcontains(msg, "no") and npcHandler.topic[cid] then
		npcHandler:say("No deal then.", cid)
		npcHandler.topic[cid] = 0
	end
	return true
end

-- Callback now set in getHandlers()


-- NpcType callbacks (MUST call setCurrentNpc first!)
npcType:eventType(NPCS_EVENT_APPEAR)
npcType:onAppear(function(npc, creature)
    setCurrentNpc(npc)
    local handlers = getHandlers(npc)
    handlers.npcHandler:onCreatureAppear(creature)
end)

npcType:eventType(NPCS_EVENT_DISAPPEAR)
npcType:onDisappear(function(npc, creature)
    setCurrentNpc(npc)
    local handlers = getHandlers(npc)
    handlers.npcHandler:onCreatureDisappear(creature)
end)

npcType:eventType(NPCS_EVENT_SAY)
npcType:onSay(function(npc, creature, type, message)
    setCurrentNpc(npc)
    local handlers = getHandlers(npc)
    handlers.npcHandler:onCreatureSay(creature, type, message)
end)

npcType:eventType(NPCS_EVENT_THINK)
npcType:onThink(function(npc, interval)
    setCurrentNpc(npc)
    local handlers = getHandlers(npc)
    handlers.npcHandler:onThink()
end)

npcType:eventType(NPCS_EVENT_CLOSECHANNEL)
npcType:onCloseChannel(function(npc, creature)
    setCurrentNpc(npc)
    local handlers = getHandlers(npc)
    handlers.npcHandler:onPlayerCloseChannel(creature)
end)

-- FocusModule now added in getHandlers()
npcType:register()
