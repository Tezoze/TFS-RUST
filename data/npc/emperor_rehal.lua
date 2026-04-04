-- Emperor Rehal - Converted from XML to Lua NpcType
-- Original XML: data/npc/Emperor Rehal.xml
-- Original Script: data/npc/scripts/Emperor Rehal.lua

local npcName = "Emperor Rehal"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a emperor rehal")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 66})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)

	if msgcontains(msg, "nokmir") then
		-- print("DEBUG: Emperor Rehal received 'nokmir', JusticeForAll = " .. player:getStorageValue(Storage.hiddenCityOfBeregar.JusticeForAll))
		if player:getStorageValue(Storage.hiddenCityOfBeregar.JusticeForAll) == 1 then
			npcHandler:say("I always liked him and I still can't believe that he really stole that ring.", cid)
			npcHandler.topic[cid] = 1
		elseif player:getStorageValue(Storage.hiddenCityOfBeregar.JusticeForAll) < 1 then
			npcHandler:say("Who? I don't know anyone by that name.", cid)
		elseif player:getStorageValue(Storage.hiddenCityOfBeregar.JusticeForAll) == 4 and player:removeItem(14348, 1) then
			npcHandler:say({
				"Interesting. The fact that you have the ring means that Nokmir can't have stolen it. Combined with the information Grombur gave you, the case appears in a completely different light. ...",
				"Let there be justice for all. Nokmir is innocent and acquitted from all charges! And Rerun... I want him in prison for this malicious act!"
			}, cid)
			player:setStorageValue(Storage.hiddenCityOfBeregar.JusticeForAll, 5)
		end
	elseif msgcontains(msg, "grombur") then
		if npcHandler.topic[cid] == 1 then
			npcHandler:say("He's very ambitious and always volunteers for the long shifts.", cid)
			player:setStorageValue(Storage.hiddenCityOfBeregar.JusticeForAll, 2)
			npcHandler.topic[cid] = 0
		end
	elseif msgcontains(msg, "promot") then
		npcHandler:say("I can promote you for 20000 gold coins. Do you want me to promote you?", cid)
		npcHandler.topic[cid] = 2
	elseif msgcontains(msg, "mission") then
		if player:getStorageValue(Storage.hiddenCityOfBeregar.RoyalRescue) < 1 then
			npcHandler:say("As you have proven yourself trustworthy I'm going to assign you a special mission. Are you interested?", cid)
			npcHandler.topic[cid] = 3
		elseif player:getStorageValue(Storage.hiddenCityOfBeregar.RoyalRescue) == 5 then
			npcHandler:say("My son was captured by trolls? Doesn't sound like him, but if you say so. Now you want a reward, huh?", cid)
			npcHandler.topic[cid] = 4
		end
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 2 then
			if player:removeMoney(20000) then
				player:setVocation(player:getVocation():getPromotion())
				npcHandler:say("Congratulations! You are now promoted.", cid)
			else
				npcHandler:say("You don't have enough money.", cid)
			end
			npcHandler.topic[cid] = 0
		elseif npcHandler.topic[cid] == 3 then
			player:setStorageValue(Storage.hiddenCityOfBeregar.RoyalRescue, 1)
			npcHandler:say({
				"Splendid! My son Rehon set off on an expedition to the deeper mines. He and a group of dwarfs were to search for new veins of crystal. Unfortunately they have been missing for 2 weeks now. ...",
				"Find my son and if he's alive bring him back. You will find a reactivated ore wagon tunnel at the entrance of the great citadel which leades to the deeper mines. If you encounter problems within the tunnel go ask Xorlosh, he can help you."
			}, cid)
			npcHandler.topic[cid] = 0
		elseif npcHandler.topic[cid] == 4 then
			player:setStorageValue(Storage.hiddenCityOfBeregar.RoyalRescue, 6)
			player:addItem(2504, 1)
			npcHandler:say("Look at these dwarven legs. They were forged years ago by a dwarf who was rather tall for our kind. I want you to have them. Thank you for rescuing my son " .. player:getName() .. ".", cid)
			npcHandler.topic[cid] = 0
		end
	elseif msgcontains(msg, "no") then
		if npcHandler.topic[cid] == 2 then
			npcHandler:say("Alright then, come back when you are ready.", cid)
			npcHandler.topic[cid] = 0
		end
	end
	return true
end

npcHandler:setMessage(MESSAGE_GREET, "May fire and earth bless you, stranger. What leads you to Beregar, the dwarven city?")
npcHandler:setMessage(MESSAGE_FAREWELL, "See you my friend.")
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
