-- Nurik - Converted from XML to Lua NpcType
-- Original XML: data/npc/Nurik.xml
-- Original Script: data/npc/scripts/Nurik.lua

local npcName = "Nurik"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a nurik")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 132, lookHead = 79, lookBody = 85, lookLegs = 86, lookFeet = 90, lookAddons = 2})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function greetCallback(cid)
	local player = Player(cid)
	if player:getStorageValue(Storage.thievesGuild.Mission04) ~= 6 or player:getOutfit().lookType ~= 66 then
		npcHandler:say('Excuse me, but I\'m waiting for someone important!', cid)
		return false
	end

	return true
end

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	if msgcontains(msg, 'dwarven bridge') then
		npcHandler:say('Wait a minute! Do I get that right? You\'re the owner of the dwarven bridge and you are willing to sell it to me??', cid)
		npcHandler.topic[cid] = 1
	elseif msgcontains(msg, 'yes') then
		if npcHandler.topic[cid] == 1 then
			npcHandler:say({
				'That\'s just incredible! I\'ve dreamed about acquiring the dwarven bridge since I was a child! Now my dream will finally become true. ...',
				'And you are sure you want to sell it? I mean really, really sure?'
			}, cid)
			npcHandler.topic[cid] = 2
		elseif npcHandler.topic[cid] == 2 then
			npcHandler:say('How splendid! Do you have the necessary documents with you?', cid)
			npcHandler.topic[cid] = 3
		elseif npcHandler.topic[cid] == 3 then
			npcHandler:say('Oh my, oh my. I\'m so excited! So let\'s seal this deal as fast as possible so I can visit my very own dwarven bridge. Are you ready for the transaction?', cid)
			npcHandler.topic[cid] = 4
		elseif npcHandler.topic[cid] == 4 then
			local player = Player(cid)
			if player:removeItem(8694, 1) then
				player:addItem(8699, 1)
				player:setStorageValue(Storage.thievesGuild.Mission04, 7)
				npcHandler:say({
					'Excellent! Here is the painting you requested. It\'s quite precious to my father, but imagine his joy when I tell him about my clever deal! ...',
					'Now leave me alone please. I have to prepare for my departure. Now my family will not call me a squandering fool anymore!'
				}, cid)
				npcHandler:releaseFocus(cid)
				npcHandler:resetNpc(cid)
			end
		end
	end
	return true
end

npcHandler:setMessage(MESSAGE_GREET, 'It\'s .. It\'s YOU! At last!! So what\'s this special proposal you would like to make, my friend?')
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
