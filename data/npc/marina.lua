-- Marina - Converted from XML to Lua NpcType
-- Original XML: data/npc/Marina.xml
-- Original Script: data/npc/scripts/Marina.lua

local npcName = "Marina"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a marina")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(0)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookTypeEx = 5811})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end
	local player = Player(cid)
	if msgcontains(msg, "silk") or msgcontains(msg, "yarn") or msgcontains(msg, "silk yarn") or msgcontains(msg, "spool of yarn") then
		if player:getStorageValue(Storage.FriendsAndTraders.TheMermaidMarina) < 1 then
			npcHandler:say("Um. You mean, you really want me to touch that gooey spider silk just because you need yarn? Well... do you think that I'm pretty?", cid)
			npcHandler.topic[cid] = 1
		elseif player:getStorageValue(Storage.FriendsAndTraders.TheMermaidMarina) == 2 then
			npcHandler:say("Okay... a deal is a deal, would you like me to create a {spool of yarn} from {10 pieces of spider silk}?", cid)
			npcHandler.topic[cid] = 5
		end
	elseif msgcontains(msg, "honey") or msgcontains(msg, "honeycomb") or msgcontains(msg, "50 honeycombs") then
		if player:getStorageValue(Storage.FriendsAndTraders.TheMermaidMarina) == 1 then
			npcHandler:say("Did you bring me the 50 honeycombs I requested and do you absolutely admire my beauty?", cid)
			npcHandler.topic[cid] = 4
		end
	elseif msgcontains(msg, "raymond striker") then
		if player:getStorageValue(Storage.TheShatteredIsles.APoemForTheMermaid) == 1 then
			npcHandler:say("<giggles> I think he has a crush on me. Well, silly man, it is only for his own good. This way he can get accustomed to TRUE beauty. And I won't give him up anymore now that he is mine.", cid)
			player:setStorageValue(Storage.TheShatteredIsles.APoemForTheMermaid, 2)
			npcHandler.topic[cid] = 0
		end
	elseif msgcontains(msg, "date") then
		if player:getStorageValue(Storage.TheShatteredIsles.ADjinnInLove) == 1 then
			npcHandler:say("Is that the best you can do? A true Djinn would have done something more poetic.", cid)
			player:setStorageValue(Storage.TheShatteredIsles.ADjinnInLove, 2)
			npcHandler.topic[cid] = 0
		elseif player:getStorageValue(Storage.TheShatteredIsles.ADjinnInLove) == 4 then
			npcHandler:say({
				"This lovely, exotic Djinn is a true poet. And he is asking me for a date? Excellent. Now I can finaly dump this human pirate. He was growing to be boring more and more with each day ...",
				"As a little reward for your efforts I allow you to ride my sea turtles. Just look around at the shores and you will find them."
			}, cid)
			player:addAchievement('Matchmaker')
			player:setStorageValue(Storage.TheShatteredIsles.ADjinnInLove, 5)
			player:setStorageValue(Storage.TheShatteredIsles.AccessToLagunaIsland, 1)
			npcHandler.topic[cid] = 0
		end
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 1 then
			npcHandler:say("Well, everyone would say that in your position. Do you think that I'm really, absolutely the most stunning being that you have ever seen?", cid)
			npcHandler.topic[cid] = 2
		elseif npcHandler.topic[cid] == 2 then
			npcHandler:say({
				"<giggles> It's funny how easy it is to get humans to say what you want. Now, proving it will be even more fun! ...",
				"You want me to touch something gooey, so you have to touch something gooey for me too. <giggles> ...",
				"I love honey and I haven't eaten it in a while, so bring me 50 honeycombs and worship my beauty a little more, then we will see."
			}, cid)
			player:setStorageValue(Storage.FriendsAndTraders.TheMermaidMarina, 1)
			player:setStorageValue(Storage.FriendsAndTraders.Questline, 1)
		elseif npcHandler.topic[cid] == 4 then
			if player:removeItem(5902, 50) then
				npcHandler:say("Oh goodie! Thank you! Okay... I guess since my fingers are sticky now anyway, I will help you. From now on, if you bring me {10 pieces of spider silk}, I will create one {spool of yarn}.", cid)
				npcHandler.topic[cid] = 0
				player:setStorageValue(Storage.FriendsAndTraders.TheMermaidMarina, 2)
			else
				npcHandler:say("You don't have enough honey.", cid)
				npcHandler.topic[cid] = 0
			end
		elseif npcHandler.topic[cid] == 5 then
			if player:removeItem(5879, 10) then
				player:addItem(5886, 1)
				npcHandler:say("Ew... gooey... there you go.", cid)
				npcHandler.topic[cid] = 0
			else
				npcHandler:say("You don't have the required items.", cid)
				npcHandler.topic[cid] = 0
			end
		end
	end
	return true
end

keywordHandler:addKeyword({'mermaid comb'}, StdModule.say, {npcHandler = npcHandler, text = 'Sorry, I don\'t have a spare comb. I lost my favourite one when diving around in Calassa.'})

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
