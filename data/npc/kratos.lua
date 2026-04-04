-- Kratos - Converted from XML to Lua NpcType
-- Original XML: data/npc/Kratos.xml
-- Original Script: data/npc/scripts/kratos.lua

local npcName = "Kratos"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a kratos")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 471, lookHead = 77, lookBody = 101, lookLegs = 97, lookFeet = 115, lookAddons = 3})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


-- ID, Count, Price
local eventShopItems = {
	["stamina refill low"] = {1000, 1, 10},
	["stamina refill medium"] = {1000, 1, 20},
	["stamina refill high"] = {1000, 1, 30},
	["blood herb"] = {2798, 10, 3}
}

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	msg = string.lower(msg)
	if (msg == "ofertas") then
		local answerOffers = ""
		for i, v in pairs(eventShopItems) do
			answerOffers = answerOffers.. " {" ..i.."} (" ..v[2].. "x) - " ..v[3].." event token(s) |"
		end
		npcHandler:say("Eu troco os itens: " ..answerOffers, cid)
	elseif (msg == "event shop") then
		npcHandler:say("Entre no nosso site, clique em {Events} => {Events Shop}.", cid)
	end
	
	if (eventShopItems[msg]) then
		npcHandler.topic[cid] = 0
		local itemId, itemCount, itemPrice = eventShopItems[msg][1], eventShopItems[msg][2], eventShopItems[msg][3]
		if (player:getItemCount(26143) > 0) then
			npcHandler:say("Deseja comprar o item {" ..msg.. "} por " ..itemPrice.. "x?", cid)
			npcHandler.topic[cid] = msg
		else
			npcHandler:say("Voc no tem " ..itemPrice.. " {Event Token(s)}!", cid)
			return true
		end
	end

	if (eventShopItems[npcHandler.topic[cid]]) then
		local itemId, itemCount, itemPrice = eventShopItems[npcHandler.topic[cid]][1], eventShopItems[npcHandler.topic[cid]][2], eventShopItems[npcHandler.topic[cid]][3]
		if (msg == "no" or
			msg == "no") then
			npcHandler:say("Ento qual item deseja comprar?", cid)
			npcHandler.topic[cid] = 0
		elseif (msg == "yes" or
				msg == "sim") then
			if (player:getItemCount(26143) > 0) then
				npcHandler:say("Voc comprou o Item {" ..npcHandler.topic[cid].."} " ..itemCount.. "x por " ..itemPrice.. " {Event Token(s)}!", cid)
				player:removeItem(26143, itemPrice)
				player:addItem(itemId, itemCount)
			end
		end

local voices = { {text = 'Troco itens por Event Tokens, venha ver minhas ofertas!'} }
npcHandler:addModule(VoiceModule:new(voices))

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
npcHandler:setMessage(MESSAGE_GREET, 'Ol, |PLAYERNAME|! Caso no me conhea, v no site e clique em {Event Shop}. Deseja trocar seus Event Tokens? fale {ofertas}.')
npcHandler:setMessage(MESSAGE_FAREWELL, 'Foi timo negociar com voc, |PLAYERNAME|.')
npcHandler:setMessage(MESSAGE_WALKAWAY, 'Foi timo negociar com voc, |PLAYERNAME|.')
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
