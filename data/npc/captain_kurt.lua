-- Captain Kurt - Converted from XML to Lua NpcType
-- Original XML: data/npc/Captain Kurt.xml
-- Original Script: data/npc/scripts/Captain Kurt.lua

local npcName = "Captain Kurt"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a captain kurt")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 96})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local towns = {
	["venore"]      = {id = 1, pos = Position(32954, 32022, 6)},
	["thais"]       = {id = 2, pos = Position(32310, 32210, 6)},
	["kazordoon"]   = {id = 3, pos = Position(32659, 31957, 15)},
	["carlin"]      = {id = 4, pos = Position(32387, 31820, 6)},
	["ab'dendriel"] = {id = 5, pos = Position(32734, 31668, 6)},
	["liberty bay"] = {id = 7, pos = Position(32285, 32892, 6)},
	["port hope"]   = {id = 8, pos = Position(32527, 32784, 6)},
	["ankrahmun"]   = {id = 9, pos = Position(33092, 32883, 6)},
	["darashia"]    = {id = 10, pos = Position(33289, 32481, 6)},
	["edron"]       = {id = 11, pos = Position(33175, 31764, 6)},
	["svargrond"]   = {id = 12, pos = Position(32341, 31108, 6)}
}

local destination = {}
local voices = { {text = 'I can take you to the mainland!'} }
npcHandler:addModule(VoiceModule:new(voices))

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	local msgLower = msg:lower()

	if npcHandler.topic[cid] == 0 then
		if msgcontains(msg, "passage") or msgcontains(msg, "travel") or msgcontains(msg, "mainland") or msgcontains(msg, "sail") or msgcontains(msg, "go") then
			npcHandler:say("In which town do you want to live: {Ab'Dendriel}, {Ankrahmun}, {Carlin}, {Darashia}, {Edron}, {Kazordoon}, {Liberty Bay}, {Port Hope}, {Svargrond}, {Thais} or {Venore}?", cid)
			npcHandler.topic[cid] = 1
		end
	elseif npcHandler.topic[cid] == 1 then
		local townData = towns[msgLower]
		if townData then
			destination[cid] = townData
			npcHandler:say("Are you sure you want to go to " .. msgLower:titleCase() .. "? This decision is irreversible. You can only take 1000 gold with you.", cid)
			npcHandler.topic[cid] = 2
		else
			npcHandler:say("In which town do you want to live: {Ab'Dendriel}, {Ankrahmun}, {Carlin}, {Darashia}, {Edron}, {Kazordoon}, {Liberty Bay}, {Port Hope}, {Svargrond}, {Thais} or {Venore}?", cid)
		end
	elseif npcHandler.topic[cid] == 2 then
		if msgcontains(msg, "yes") then
			local totalMoney = player:getMoney()
			if totalMoney > 1000 then
				npcHandler:say("You have " .. totalMoney .. " gold with you. You can only take 1000 gold. Please deposit the rest at the bank (Raffael).", cid)
				npcHandler.topic[cid] = 0
			else
				local townInfo = destination[cid]
				player:setTown(Town(townInfo.id))
				player:teleportTo(townInfo.pos)
				player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
				npcHandler:say("Bon voyage!", cid)
				npcHandler:resetNpc(cid)
			end
		else
			 npcHandler:say("Where do you want to go then?", cid)
			 npcHandler.topic[cid] = 1
		end
	end
	return true
end

npcHandler:setMessage(MESSAGE_GREET, "Hello |PLAYERNAME|. I can take you to the {mainland}.")
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
