-- Lamassut - Converted from XML to Lua NpcType
-- Original XML: data/npc/Lamassut.xml
-- Original Script: data/npc/scripts/spells/healing spell.lua

local npcName = "Lamassut"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a lamassut")
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
	[8704]={ buy =1, spell = "Light Healing", name = "Light Healing", vocations = {1,2,3,5,6,7}, level = 8, premium = 0},
	[8474]={ buy =150, spell = "Cure Poison", name = "Cure Poison", vocations = {1,2,3,4,5,6,7,8}, level = 10, premium = 0},	
	[7488]={ buy =800, spell = "Heal Friend", name = "Heal Friend", vocations = {2,6}, level = 18, premium = 0},
	[2265]={ buy =350, spell = "Intense Healing", name = "Intense Healing", vocations = {1,2,3,5,6,7}, level = 20, premium = 0},	
	[2273]={ buy =1000, spell = "Ultimate Healing", name = "Ultimate Healing", vocations = {1,2,5,6}, level = 30, premium = 0},	
	[7588]={ buy =3000, spell = "Divine Healing", name = "Divine Healing", vocations = {3,7}, level = 35, premium = 0},
	[8919]={ buy =2200, spell = "Mass Healing", name = "Mass Healing", vocations = {2,6}, level = 36, premium = 0},
	[2640]={ buy =10000, spell = "Intense Recovery", name = "Intense Recovery", vocations = {3,4,7,8}, level = 100, premium = 0}
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
        npcHandler:say("Here are the spells that you can learn from me.", cid)
        for var, item in pairs(spells) do
            if not AllSpells then
                if not player:hasLearnedSpell(item.spell) then
                    if player:getLevel() >= item.level then
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
