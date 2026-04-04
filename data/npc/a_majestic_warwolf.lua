-- A Majestic Warwolf - Converted from XML to Lua NpcType
-- Original XML: data/npc/A Majestic Warwolf.xml
-- Original Script: data/npc/scripts/A Majestic Warwolf.lua

local npcName = "A Majestic Warwolf"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a a majestic warwolf")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 3})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function greetCallback(cid)
	if Player(cid):getStorageValue(Storage.OutfitQuest.DruidHatAddon) < 9 then
		npcHandler:say('GRRRRRRRRRRRRR', cid)
		return false
	end
	return true
end

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end
	local player = Player(cid)
	if isInArray({'addon', 'outfit'}, msg) then
		if player:getStorageValue(Storage.OutfitQuest.DruidHatAddon) == 9 then
			npcHandler:say('I can see in your eyes that you are a honest and friendly person, |PLAYERNAME|. You were patient enough to learn our language and I will grant you a special gift. Will you accept it?', cid)
			npcHandler.topic[cid] = 1
		end
	elseif msgcontains(msg, 'yes') and npcHandler.topic[cid] == 1 then
		player:setStorageValue(Storage.OutfitQuest.DruidHatAddon, 10)
		player:addOutfitAddon(148, 2)
		player:addOutfitAddon(144, 2)
		player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
		npcHandler:say(player:getSex() == PLAYERSEX_FEMALE and 'From now on, you shall be known as |PLAYERNAME|, the wolf girl. You shall be fast and smart as Morgrar, the great white wolf. He shall guide your path.' or 'From now on, you shall be known as |PLAYERNAME|, the bear warrior. You shall be strong and proud as Angros, the great dark bear. He shall guide your path.', cid)
		npcHandler.topic[cid] = 0
	end
	return true
end

npcHandler:setCallback(CALLBACK_GREET, greetCallback)
npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
npcHandler:setMessage(MESSAGE_GREET, "Interesting. A human who can speak the language of wolves.")
npcHandler:setMessage(MESSAGE_FAREWELL, "YOOOOUHHOOOUU!")
npcHandler:setMessage(MESSAGE_WALKAWAY, "YOOOOUHHOOOUU!")


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
