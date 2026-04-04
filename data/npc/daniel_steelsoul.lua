-- Daniel Steelsoul - Converted from XML to Lua NpcType
-- Original XML: data/npc/Daniel Steelsoul.xml
-- Original Script: data/npc/scripts/Daniel Steelsoul.lua

local npcName = "Daniel Steelsoul"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a daniel steelsoul")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 73})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local condition = Condition(CONDITION_FIRE)
condition:setParameter(CONDITION_PARAM_DELAYED, 1)
condition:addDamage(14, 1000, -10)

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end
	local player = Player(cid)
	if isInArray({"fuck", "idiot", "asshole", "ass", "fag", "stupid", "tyrant", "shit", "lunatic"}, msg) then
		npcHandler:say("Take this!", cid)
		player:getPosition():sendMagicEffect(CONST_ME_EXPLOSIONAREA)
		player:addCondition(condition)
		npcHandler:releaseFocus(cid)
		npcHandler:resetNpc(cid)
	elseif msgcontains(msg, "mission") then
		if player:getStorageValue(Storage.TibiaTales.AgainstTheSpiderCult) < 1 then
			npcHandler.topic[cid] = 1
			npcHandler:say("Very good, we need heroes like you to go on a suici.....er....to earn respect of the authorities here AND in addition get a great reward for it. Are you interested in the job?", cid)
		elseif player:getStorageValue(Storage.TibiaTales.AgainstTheSpiderCult) == 5 then
			player:setStorageValue(Storage.TibiaTales.AgainstTheSpiderCult, 6)
			npcHandler.topic[cid] = 0
			player:addItem(7887, 1)
			npcHandler:say("What? YOU DID IT?!?! That's...that's...er....<drops a piece of paper. You see the headline 'death certificate'> like I expected!! Here is your reward.", cid)
		end
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 1 then
			player:setStorageValue(Storage.TibiaTales.DefaultStart, 1)
			player:setStorageValue(Storage.TibiaTales.AgainstTheSpiderCult, 1)
			npcHandler:say({
				"Very well, maybe you know that the orcs here in Edron learnt to raise giant spiders. It is going to become a serious threat. ...",
				"The mission is simple: go to the orcs and destroy all spider eggs that are hatched by the giant spider they have managed to catch. The orcs are located in the south of the western part of the island."
			}, cid)
		end
	end
	return true
end

keywordHandler:addKeyword({'job'}, StdModule.say, {npcHandler = npcHandler, text = "I am the governor of this isle, Edron, and grandmaster of the Knights of Banor's Blood."})
keywordHandler:addKeyword({'king'}, StdModule.say, {npcHandler = npcHandler, text = "LONG LIVE THE KING!"})

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
npcHandler:setMessage(MESSAGE_GREET, "Greetings and Banor be with you, |PLAYERNAME|!")
npcHandler:setMessage(MESSAGE_FAREWELL, "PRAISE TO BANOR!")
npcHandler:setMessage(MESSAGE_WALKAWAY, "PRAISE TO BANOR!")


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
