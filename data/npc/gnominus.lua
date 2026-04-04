-- Gnominus - Converted from XML to Lua NpcType
-- Original XML: data/npc/Gnominus.xml
-- Original Script: data/npc/scripts/Gnominus.lua

local npcName = "Gnominus"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a gnominus")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 493, lookBody = 85, lookLegs = 85})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


-- transcript for buying fresh mushroom beer is probably wrong except for the case where you buy it
local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end
	local player = Player(cid)
	if msgcontains(msg, 'recruitment') then
		if player:getStorageValue(Storage.BigfootBurden.QuestLine) == 3 then
			npcHandler:say('Your examination is quite easy. Just step through the green crystal apparatus in the south! We will examine you with what we call g-rays. Where g stands for gnome of course ...', cid)
			npcHandler:say('Afterwards walk up to Gnomedix for your ear examination.', cid)
			npcHandler.topic[cid] = 1
		end
	elseif msgcontains(msg, 'tavern') then
			npcHandler:say('I provide the population with some fresh alcohol-free mushroom {beer}!', cid)
	elseif msgcontains(msg, 'beer') then
			npcHandler:say('Do you want some mushroom beer for 10 gold?', cid)
			npcHandler.topic[cid] = 2
	elseif npcHandler.topic[cid] == 1 then
		if msgcontains(msg, 'apparatus') then
			npcHandler:say('Don\'t be afraid. It won\'t hurt! Just step in!', cid)
			player:setStorageValue(Storage.BigfootBurden.QuestLine, 4)
			npcHandler.topic[cid] = 0
		end
	elseif npcHandler.topic[cid] == 2 then
		if msgcontains(msg, 'yes') then
			if player:getMoney() + player:getBankBalance() >= 10 then
				npcHandler:say('And here it is! Drink it quick, it gets stale quite fast!', cid)
				player:removeMoneyNpc(10)
				local beerItem = player:addItem(18305)
				if beerItem then
					beerItem:decay()
				end
			else
				npcHandler:say('You do not have enough money.', cid)
			end
		else
			npcHandler:say('Come back later.', cid)
		npcHandler.topic[cid] = 0
	return true

end
npcHandler:setMessage(MESSAGE_GREET, 'Hi there! Welcome to my little {tavern}.')
npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
end
end


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
