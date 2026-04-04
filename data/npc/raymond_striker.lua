-- Raymond Striker - Converted from XML to Lua NpcType
-- Original XML: data/npc/Raymond Striker.xml
-- Original Script: data/npc/scripts/Raymond Striker.lua

local npcName = "Raymond Striker"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a raymond striker")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 151, lookHead = 39, lookBody = 77, lookLegs = 98, lookFeet = 95, lookAddons = 1})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end
	local player = Player(cid)
	if msgcontains(msg, "eleonore") then
		if player:getStorageValue(Storage.TheShatteredIsles.APoemForTheMermaid) < 1 then
			npcHandler:say("Eleonore ... Yes, I remember her... vaguely. She is a pretty girl ... but still only a girl and now I am in love with a beautiful and passionate woman. A true {mermaid} even.", cid)
			npcHandler.topic[cid] = 1
		end
	elseif msgcontains(msg, "mission") then
		if player:getStorageValue(Storage.TheShatteredIsles.APoemForTheMermaid) < 1 then
			npcHandler:say("Don't ask about silly missions. All I can think about is this lovely {mermaid}.", cid)
			npcHandler.topic[cid] = 1
		end
	elseif msgcontains(msg, "mermaid") then
		if npcHandler.topic[cid] == 1 then
			npcHandler:say("The mermaid is the most beautiful creature I have ever met. She is so wonderful. It was some kind of magic as we first met. A look in her eyes and I suddenly knew there would be never again another woman in my life but her.", cid)
			npcHandler.topic[cid] = 0
			player:setStorageValue(Storage.TheShatteredIsles.APoemForTheMermaid, 1)
		end
	elseif msgcontains(msg, "pirate outfit") then
		if player:getStorageValue(Storage.TheShatteredIsles.AccessToLagunaIsland) == 1 and player:getStorageValue(Storage.OutfitQuest.PirateBaseOutfit) < 1 then
			npcHandler:say("Ah, right! The pirate outfit! Here you go, now you are truly one of us.", cid)
			player:addOutfit(151)
			player:addOutfit(155)
			player:getPosition():sendMagicEffect(CONST_ME_MAGIC_GREEN)
			player:setStorageValue(Storage.OutfitQuest.PirateBaseOutfit, 1)
			npcHandler.topic[cid] = 0
		else
			npcHandler:say("test: ".. player:getStorageValue(Storage.OutfitQuest.PirateBaseOutfit), cid)
		end
	elseif msgcontains(msg, "task") or msgcontains(msg, "mission") then
		if player:getStorageValue(Storage.TheShatteredIsles.APoemForTheMermaid) >= 1 then
			if player:getStorageValue(Storage.KillingInTheNameOf.RaymondPirates) <= 0 then
				npcHandler:say({
					"Ah, a true pirate at heart! Those scurvy dogs have been plaguing the seas for too long. ...",
					"If you want to prove yourself as a real pirate, I have a task for you. Kill {3000 pirates} for me - make them walk the plank! Are you up for it?"
				}, cid)
				npcHandler.topic[cid] = 2
			elseif player:getStorageValue(Storage.KillingInTheNameOf.RaymondPirates) == 1 then
				if player:getStorageValue(Storage.KillingInTheNameOf.RaymondPiratesCount) >= 3000 then
					npcHandler:say({
						"By Blackbeard's beard! You've sent 3000 pirates to Davy Jones' locker! You're a true pirate legend! ...",
						"As a reward, I'll let you challenge Lethal Lissy herself. She's the most fearsome pirate captain alive. Find her hideout and show her what a real pirate can do!"
					}, cid)
					player:setStorageValue(17523, 1) -- Access to Lethal Lissy boss
					player:setStorageValue(Storage.KillingInTheNameOf.RaymondPirates, 2)
				else
					npcHandler:say("Come back when you've sent {3000 pirates} to the depths!", cid)
				end
			elseif player:getStorageValue(Storage.KillingInTheNameOf.RaymondPirates) == 2 then
				npcHandler:say("You've already completed my task, matey. But if you want more pirate action, talk to Grizzly Adams about other hunting tasks.", cid)
			end
		else
			npcHandler:say("I don't have any tasks for landlubbers. Complete my {mermaid} quest first!", cid)
		end
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 2 then
			npcHandler:say("Excellent! Go show those pirates what a real buccaneer can do!", cid)
			player:setStorageValue(Storage.KillingInTheNameOf.Join, 1)
			player:setStorageValue(Storage.KillingInTheNameOf.RaymondPirates, 1)
			player:setStorageValue(Storage.KillingInTheNameOf.RaymondPiratesCount, 0)
			npcHandler.topic[cid] = 0
		end
	elseif msgcontains(msg, "no") then
		if npcHandler.topic[cid] == 2 then
			npcHandler:say("Suit yourself, matey. But true pirates don't back down from a fight!", cid)
			npcHandler.topic[cid] = 0
		end
	end
	return true
end

npcHandler:setMessage(MESSAGE_GREET, "Be greeted. Is there anything I can {do for you}?")
npcHandler:setMessage(MESSAGE_FAREWELL, "Good bye.")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Oh well.")

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
