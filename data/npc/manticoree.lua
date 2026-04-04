-- Manticoree - Converted from XML to Lua NpcType
-- Original XML: data/npc/Manticoree.xml
-- Original Script: data/npc/scripts/spells/support spells.lua

local npcName = "Manticoree"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a manticoree")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 319})
npcType:speechBubble(SPEECHBUBBLE_TRADE)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local voices = { {text = 'Hello there, adventurer! Need a deal in spells? I\'m your man!'} }
npcHandler:addModule(VoiceModule:new(voices))

local PremiumSpells = false
local AllSpells = false
-- 1,5 Sorcerer
-- 2,6 Druid
-- 3,7 Paladin
-- 4,8 Knight

local spells = {
	[2050]={ buy =1, spell = "Light", name = "Light", vocations = {1,2,3,4,5,6,7,8}, level = 1, premium = 0, QuestPoints = 0},
	[32605]={ buy =80, spell = "Find Person", name = "Find Person", vocations = {1,2,3,4,5,6,7,8}, level = 1, premium = 0, QuestPoints = 0},
	[2120]={ buy =200, spell = "Magic Rope", name = "Magic Rope", vocations = {1,2,3,4,5,6,7,8}, level = 9, premium = 0, QuestPoints = 0},
	[1386]={ buy =500, spell = "Levitate", name = "Levitate", vocations = {1,2,3,4,5,6,7,8}, level = 12, premium = 0, QuestPoints = 0},
	[2051]={ buy =500, spell = "Great Light", name = "Great Light", vocations = {1,2,3,4,5,6,7,8}, level = 13, premium = 0, QuestPoints = 0},
	[1987]={ buy =100, spell = "Empty Bag", name = "Empty Bag", vocations = {1,2,3,4,5,6,7,8}, level = 8, premium = 0, QuestPoints = 10},
	[2169]={ buy =600, spell = "Haste", name = "Haste", vocations = {1,2,3,4,5,6,7,8}, level = 14, premium = 0, QuestPoints = 0},
	[2523]={ buy =450, spell = "Magic Shield", name = "Magic Shield", vocations = {1,2,5,6}, level = 14, premium = 0, QuestPoints = 0},
	[2206]={ buy =1300, spell = "Strong Haste", name = "Strong Haste", vocations = {1,2,5,6}, level = 20, premium = 0, QuestPoints = 0},
	[5787]={ buy =2000, spell = "Challenge", name = "Challenge", vocations = {8}, level = 20, premium = 0, QuestPoints = 0},
	[12636]={ buy =1000, spell = "Creature Illusion", name = "Creature Illusion", vocations = {1,2,5,6}, level = 23, premium = 0, QuestPoints = 0},
	[9007]={ buy =2000, spell = "Summon Creature", name = "Summon Creature", vocations = {1,2,5,6}, level = 25, premium = 0, QuestPoints = 0},
	[2195]={ buy =1300, spell = "Charge", name = "Charge", vocations = {4,8}, level = 25, premium = 0, QuestPoints = 0},
	[2163]={ buy =1600, spell = "Ultimate Light", name = "Ultimate Light", vocations = {1,2,5,6}, level = 26, premium = 0, QuestPoints = 0},
	[2202]={ buy =1600, spell = "Cancel Invisibility", name = "Cancel Invisibility", vocations = {3,7}, level = 26, premium = 0, QuestPoints = 0},
	[2165]={ buy =2000, spell = "Invisibility", name = "Invisibility", vocations = {1,2,3,5,6,7}, level = 35, premium = 0, QuestPoints = 0},
	[2522]={ buy =6000, spell = "Protector", name = "Protector", vocations = {4,8}, level = 55, premium = 0, QuestPoints = 0},
	[11303]={ buy =6000, spell = "Swift Foot", name = "Swift Foot", vocations = {3,7}, level = 55, premium = 0, QuestPoints = 0},
	[7416]={ buy =8000, spell = "Blood Rage", name = "Blood Rage", vocations = {4,8}, level = 60, premium = 0, QuestPoints = 0},
	[5777]={ buy =8000, spell = "Sharpshooter", name = "Sharpshooter", vocations = {3,7}, level = 60, premium = 0, QuestPoints = 0},
	[2671]={ buy =300, spell = "Food", name = "Food", vocations = {2,6}, level = 14, premium = 0, QuestPoints = 0}
}


local mensaje
function creatureSayCallback(cid, type, msg)
    if not npcHandler:isFocused(cid) then
        return false
    end
		
    local shopWindow = {}
    local player = Player(cid)

    local function onBuy(cid, item, subType, amount, ignoreCap, inBackpacks)		
		
        local player = Player(cid)		
        if player:hasLearnedSpell(spells[item].spell) then
			return false
        end       
        if player:getLevel() < spells[item].level then		
            return false
        end
        if not isInArray(spells[item].vocations, player:getVocation():getId()) then			
            return false
        end
        if PremiumSpells and (spells[item].premium == 1) and not player:isPremium() then
            return false
        end   
		
        if player:removeMoneyNpc(spells[item].buy) == false then
            return false
        end
		
        player:learnSpell(spells[item].spell)
        player:getPosition():sendMagicEffect(12)
        return true
    end

    if msgcontains(msg, "spells") then
	
	local QPoints = 0	
	local points = db.storeQuery("SELECT `QuestPoints` FROM `players` WHERE `id` = " .. getPlayerGUID(player))	
    if points then
        QPoints = result.getDataInt(points, "QuestPoints")
    end
	
        npcHandler:say("Here are the spells that you can learn from me.", cid)
        for var, item in pairs(spells) do
            if not AllSpells then
                if not player:hasLearnedSpell(item.spell) then
                    if (player:getLevel() >= item.level and QPoints >= item.QuestPoints) then
                        if isInArray(item.vocations, player:getVocation():getId()) then
                            if PremiumSpells then
                                if (item.premium == 1) and player:isPremium() then
                                    table.insert(shopWindow, {id = var, subType = 0, buy = item.buy, sell = 0, name = item.name})
                                end
                            else
                                table.insert(shopWindow, {id = var, subType = 0, buy = item.buy, sell = 0, name = item.name})
                            end
                        end
                    end
                end
            else
                table.insert(shopWindow, {id = var, subType = 0, buy = item.buy, sell = 0, name = item.name})
            end
        end
        openShopWindow(cid, shopWindow, onBuy, onSell)
    end
	
    return true
end



npcHandler:setMessage(MESSAGE_GREET, "Welcome to my shop, adventurer |PLAYERNAME|! I sell {Spells}.")
npcHandler:setMessage(MESSAGE_FAREWELL, "Good bye and come again, |PLAYERNAME|.")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Good bye and come again.")

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
