-- Awareness of the Emperor - Converted from XML to Lua NpcType
-- Original XML: data/npc/Awareness of the Emperor.xml
-- Original Script: data/npc/scripts/Awareness of the Emperor.lua

local npcName = "Awareness of the Emperor"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a awareness of the emperor")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 231})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	if msgcontains(msg, "mission") then
		local player = Player(cid)
		local questline = player:getStorageValue(Storage.WrathoftheEmperor.Questline)
		local bossStatus = player:getStorageValue(Storage.WrathoftheEmperor.BossStatus)
		print("DEBUG: Awareness NPC - Player said 'mission', Questline: " .. questline .. ", BossStatus: " .. bossStatus)

		if player:getStorageValue(Storage.WrathoftheEmperor.Questline) == 30 and player:getStorageValue(Storage.WrathoftheEmperor.BossStatus) == 5 then
			npcHandler:say({
				"The amplified force of the snake god is tearing the land apart. It is using my crystals in a reverse way to drain the vital force from the land and its inhabitants to fuel its power. ...",
				"I will withstand its influence as good as possible and slow this process. You will have to fight its worldly incarnation though. ...",
				"It is still weak and disoriented. You might stand a chance - this is our only chance. I will send you to the point to where the vital force is channelled. I have no idea where that might be though. ...",
				"You will probably have to fight some sort of vessel the snake god uses. Even if you defeat it, it is likely that it only weakens the snake. ...",
				"You might have to fight several incarnations until the snake god is worn out enough. Then use the power of the snake's own sceptre against it. Use it on its corpse to claim your victory. ...",
				"Be prepared for the fight of your life! Are you ready?"
			}, cid)
			npcHandler.topic[cid] = 1
		elseif player:getStorageValue(Storage.WrathoftheEmperor.Questline) == 32 then
			print("DEBUG: Awareness NPC - Giving final reward to player")
			npcHandler:say({
				"So you have mastered the crisis you invoked with your foolishness. I should crush you for your involvement right here and now. ...",
				"But such an act would bring me down to your own barbaric level and only fuel the corruption that destroys the land that I own. Therefore I will not only spare your miserable life but show your the generosity of the dragon emperor. ...",
				"I will reward you beyond your wildest dreams! ...",
				"I grant you three chests - filled to the lid with platinum coins, a house in the city in which you may reside, a set of the finest armor Zao has to offer, and a casket of never-ending mana. ...",
				"Speak with magistrate Izsh in the ministry about your reward. And now leave before I change my mind!"
			}, cid)
			player:setStorageValue(Storage.WrathoftheEmperor.Questline, 33)
			player:setStorageValue(Storage.WrathoftheEmperor.Mission12, 0) --Questlog, Wrath of the Emperor "Mission 12: Just Rewards"
		end
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 1 then
			local player = Player(cid)
			player:setStorageValue(Storage.WrathoftheEmperor.Questline, 31)
			player:setStorageValue(Storage.WrathoftheEmperor.Mission11, 0) -- Initialize quest log
			player:setStorageValue(Storage.WrathoftheEmperor.Mission11, 1) --Questlog, Wrath of the Emperor "Mission 11: Payback Time"
			player:registerEvent("WotEZalamon") -- Dynamic registration for Zalamon events
			npcHandler:say("So be it!", cid)
			npcHandler.topic[cid] = 0
		end
	end
	return true
end

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
