-- Melchior - Converted from XML to Lua NpcType
-- Original XML: data/npc/Melchior.xml
-- Original Script: data/npc/scripts/Melchior.lua

local npcName = "Melchior"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a melchior")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 130, lookBody = 25, lookLegs = 59, lookFeet = 115})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function greetCallback(cid)
	npcHandler:setMessage(MESSAGE_GREET, Player(cid):getSex() == PLAYERSEX_FEMALE and 'Welcome, |PLAYERNAME|! The lovely sound of your voice shines like a beam of light through my solitary darkness!' or 'Greetings, |PLAYERNAME|. I do not see your face, but I can read a thousand things in your voice!')
	return true
end

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	if msgcontains(msg, 'greeting') then
		if player:getStorageValue(Storage.DjinnWar.Faction.Greeting) ~= 1 then
			npcHandler:say({
				'The djinns have an ancient code of honour. This code includes a special concept of hospitality. Anybody who utters the word of greeting must not be attacked even if he is an enemy. Well, at least that is what the code says. ...',
				'I have found out, though, that this does not work at all times. There is no point to say the word of greeting to an enraged djinn. ...',
				'I can tell you the word of greeting if you\'re interested. It is {DJANNI\'HAH}. Remember this word well, stranger. It might save your life one day. ...',
				'And keep in mind that you must choose sides in this conflict. You can only follow the Efreet or the Marid - once you have made your choice there is no way back. I know from experience that djinn do not tolerate double-crossing.'
			}, cid)

			player:setStorageValue(Storage.DjinnWar.Faction.Greeting, 1)
		end
	end

	return true
end

npcHandler:setMessage(MESSAGE_FAREWELL, 'Farewell, stranger. May Uman the Wise guide your steps in this treacherous land.')
npcHandler:setMessage(MESSAGE_WALKAWAY, 'Farewell, stranger. May Uman the Wise guide your steps in this treacherous land.')

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
