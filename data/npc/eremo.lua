-- Eremo - Converted from XML to Lua NpcType
-- Original XML: data/npc/Eremo.xml
-- Original Script: data/npc/scripts/Eremo.lua

local npcName = "Eremo"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a eremo")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 130, lookBody = 109, lookLegs = 128, lookFeet = 128})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)

-- PENDING TRANSACTION TABLES
local pendingBless = {}
local pendingSpell = {}



-- SPELL CONFIGURATION
local spells = {
    {name = 'Challenge',           price = 2000, level = 20, vocations = {4, 8}, keywords = {'challenge'}},
    {name = 'Conjure Power Bolt',  price = 2000, level = 59, vocations = {3, 7}, keywords = {'conjure', 'power', 'bolt'}},
    {name = 'Enchant Staff',       price = 2000, level = 41, vocations = {1, 5}, keywords = {'enchant', 'staff'}},
    {name = 'Wild Growth',         price = 2000, level = 27, vocations = {2, 6}, keywords = {'wild', 'growth'}}
}

-- HELPER FUNCTIONS
local function hasVocation(player, allowedList)
    local vid = player:getVocation():getId()
    for _, allowed in ipairs(allowedList) do
        if vid == allowed then return true end
    end
    return false
end

-- BLESSING SYSTEM
local function confirmBlessingPurchase(cid)
    local player = Player(cid)
    local blessData = pendingBless[cid]
    if not blessData then return false end
    
    if player:hasBlessing(5) then
        npcHandler:say("Gods have already blessed you with this blessing!", cid)
        pendingBless[cid] = nil
        return true
    end
    
    if not player:removeTotalMoney(blessData.cost) then
        npcHandler:say("You don't have enough money for blessing.", cid)
        pendingBless[cid] = nil
        return true
    end
    
    player:addBlessing(5)
    player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
    
    if blessData.isPilgrimage then
        player:setStorageValue(Storage.PilgrimageOfAshes.Mission05, 2)
        player:setStorageValue(Storage.PilgrimageOfAshes.Questline, 6)
        npcHandler:say({
            "So receive the wisdom of solitude, pilgrim. This is the last of five available blessings. If you have all of them when bad fate befalls you your losses will be minimal. ...",
            "You can complete your pilgrimage at any city guide. I wish you the best of luck and a blessed journey, pilgrim. Visit me again sometime."
        }, cid)
    else
        npcHandler:say("So receive the wisdom of solitude, pilgrim.", cid)
    end
    
    pendingBless[cid] = nil
    return true
end

-- SPELL SYSTEM
local function confirmSpellPurchase(cid)
    local player = Player(cid)
    local spellData = pendingSpell[cid]
    if not spellData then return false end
    
    -- Re-validate before purchase (player could have leveled/changed in meantime)
    if not hasVocation(player, spellData.vocations) then
        npcHandler:say('This spell is not for your vocation.', cid)
        pendingSpell[cid] = nil
        return true
    end
    
    if player:getLevel() < spellData.level then
        npcHandler:say('You must be at least level ' .. spellData.level .. ' to learn this spell.', cid)
        pendingSpell[cid] = nil
        return true
    end
    
    if player:getTotalMoney() < spellData.price then
        npcHandler:say('You don\'t have enough money. This spell costs ' .. spellData.price .. ' gold.', cid)
        pendingSpell[cid] = nil
        return true
    end
    
    player:removeTotalMoney(spellData.price)
    player:learnSpell(spellData.name)
    
    npcHandler:say('You have learned ' .. spellData.name .. ' for ' .. spellData.price .. ' gold!', cid)
    player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
    pendingSpell[cid] = nil
    return true
end

local function offerSpell(cid, spellName, price, level, vocations)
    local player = Player(cid)
    
    if player:hasLearnedSpell(spellName) then
        npcHandler:say('You already know this spell.', cid)
        return true
    elseif player:getLevel() < level then
        npcHandler:say('You must be at least level ' .. level .. ' to learn this spell.', cid)
        return true
    elseif not hasVocation(player, vocations) then
        npcHandler:say('This spell is not for your vocation.', cid)
        return true
    elseif player:getTotalMoney() < price then
        npcHandler:say('You don\'t have enough money. This spell costs ' .. price .. ' gold.', cid)
        return true
    end
    
    pendingSpell[cid] = {name = spellName, price = price, level = level, vocations = vocations}
    npcHandler:say('Would you like to purchase the ' .. spellName .. ' spell for ' .. price .. ' gold?', cid)
    npcHandler.topic[cid] = 999
    return true
end

-- MAIN CALLBACK
local function creatureSayCallback(cid, type, msg)
    if not npcHandler:isFocused(cid) then
        return false
    end
    
    local player = Player(cid)
    if not player then
        return false
    end
    
    local m = msg:lower()
    
    -- Handle confirmations
    if npcHandler.topic[cid] == 999 then
        if m == 'yes' then 
            confirmSpellPurchase(cid)
        elseif m == 'no' then 
            npcHandler:say('Maybe another time then.', cid)
            pendingSpell[cid] = nil
        end
        npcHandler.topic[cid] = 0
        return true
    end
    
    if npcHandler.topic[cid] == 1 then -- Regular blessing confirmation
        if m == 'yes' then
            confirmBlessingPurchase(cid)
        elseif m == 'no' then
            npcHandler:say('Fine. You are free to decline my offer.', cid)
            pendingBless[cid] = nil
        end
        npcHandler.topic[cid] = 0
        return true
    end
    
    if npcHandler.topic[cid] == 2 then -- Pilgrimage blessing confirmation
        if m == 'yes' then
            confirmBlessingPurchase(cid)
        elseif m == 'no' then
            npcHandler:say('Fine. You are free to decline my offer.', cid)
            pendingBless[cid] = nil
        end
        npcHandler.topic[cid] = 0
        return true
    end
    
    if npcHandler.topic[cid] == 3 then -- Twist of Fate confirmation
        if m == 'yes' then
            local pvpBlessCost = StdModule.calculateTwistOfFateBlessingCost(player:getLevel())
            
            if player:hasBlessing(6) then
                npcHandler:say("Gods have already blessed you with this blessing!", cid)
            elseif not player:removeTotalMoney(pvpBlessCost) then
                npcHandler:say("You don't have enough money for blessing.", cid)
            else
                player:addBlessing(6)
                npcHandler:say("So receive the protection of the twist of fate, pilgrim.", cid)
                player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
            end
        elseif m == 'no' then
            npcHandler:say('Fine. You are free to decline my offer.', cid)
        end
        npcHandler.topic[cid] = 0
        return true
    end
    
    if npcHandler.topic[cid] == 4 then -- Teleport confirmation (CHANGED FROM 3 TO 4)
        if m == 'yes' then
            player:teleportTo(Position(33288, 31956, 6))
            player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
            npcHandler:say('Here you go!', cid)
        else
            npcHandler:say('Maybe later.', cid)
        end
        npcHandler.topic[cid] = 0
        return true
    end
    
    -- Healing
    if m:find('heal') then
        local healed = false
        
        if player:getCondition(CONDITION_FIRE) then
            npcHandler:say("You are burning. Let me quench those flames.", cid)
            player:removeCondition(CONDITION_FIRE)
            player:getPosition():sendMagicEffect(CONST_ME_MAGIC_GREEN)
            healed = true
        elseif player:getCondition(CONDITION_POISON) then
            npcHandler:say("You are poisoned. Let me soothe your pain.", cid)
            player:removeCondition(CONDITION_POISON)
            player:getPosition():sendMagicEffect(CONST_ME_MAGIC_RED)
            healed = true
        elseif player:getCondition(CONDITION_ENERGY) then
            npcHandler:say("You are electrified, my child. Let me help you to stop trembling.", cid)
            player:removeCondition(CONDITION_ENERGY)
            player:getPosition():sendMagicEffect(CONST_ME_MAGIC_GREEN)
            healed = true
        elseif player:getHealth() < 40 then
            npcHandler:say("You are hurt, my child. I will heal your wounds.", cid)
            local health = player:getHealth()
            if health < 40 then
                player:addHealth(40 - health)
            end
            player:getPosition():sendMagicEffect(CONST_ME_MAGIC_GREEN)
            healed = true
        end
        
        if not healed then
            npcHandler:say("You aren't looking that bad. Sorry, I can't help you. But if you are looking for additional protection you should go on the {pilgrimage} of ashes or get the protection of the {twist of fate} here.", cid)
        end
        
        return true
    end
    
    -- Twist of Fate info
    if m:find('twist') or m:find('fate') then
        local pvpBlessCost = StdModule.calculateTwistOfFateBlessingCost(player:getLevel())
        
        if player:hasBlessing(6) then
            npcHandler:say("Gods have already blessed you with this blessing!", cid)
            return true
        end
        
        -- Check if player has at least one other blessing
        local hasOtherBlessing = false
        for i = 1, 5 do
            if player:hasBlessing(i) then
                hasOtherBlessing = true
                break
            end
        end
        
        if not hasOtherBlessing then
            npcHandler:say("You need to have at least one of the other five blessings before you can receive the twist of fate.", cid)
            return true
        end
        
        npcHandler:say({
            'This is a special blessing I can bestow upon you once you have obtained at least one of the other blessings and which functions a bit differently. ...',
            'It only works when you\'re killed by other adventurers, which means that at least forty percent of the damage leading to your death was caused by others, not by monsters or the environment. ...',
            'The twist of fate will not reduce the death penalty like the other blessings, but instead prevent you from losing your other blessings as well as the amulet of loss, should you wear one. ...',
            'Would you like to receive that protection for a sacrifice of ' .. pvpBlessCost .. ' gold, child?'
        }, cid)
        npcHandler.topic[cid] = 3
        return true
    end
    
    -- Regular blessing (Wisdom of Solitude)
    if m:find('solitude') or m:find('wisdom') then
        if player:hasBlessing(5) then
            npcHandler:say("Gods have already blessed you with this blessing!", cid)
            return true
        end
        
        local blessCost = StdModule.calculateRegularBlessingCost(player:getLevel())
        pendingBless[cid] = {cost = blessCost, isPilgrimage = false}
        npcHandler:say('Would you like to receive that protection for a sacrifice of ' .. blessCost .. ' gold, child?', cid)
        npcHandler.topic[cid] = 1
        return true
    end
    
    -- Pilgrimage mission
    if m:find('mission') then
        if player:getStorageValue(Storage.PilgrimageOfAshes.Questline) < 1 then
            npcHandler:say("I sense you are not on the pilgrimage of ashes. You should start this quest with one of the city guides first.", cid)
            return true
        end
        
        if player:getStorageValue(Storage.PilgrimageOfAshes.Mission05) >= 2 then
            npcHandler:say("You have already received the wisdom of solitude. You should continue your pilgrimage to the next sacred place.", cid)
            return true
        end
        
        if player:hasBlessing(5) then
            npcHandler:say("Gods have already blessed you with this blessing!", cid)
            return true
        end
        
        local blessCost = StdModule.calculateRegularBlessingCost(player:getLevel()) - 1000
        pendingBless[cid] = {cost = blessCost, isPilgrimage = true}
        npcHandler:say('Pilgrim, you have come far on your journey. Would you like to receive the wisdom of solitude for ' .. blessCost .. ' gold coins?', cid)
        npcHandler.topic[cid] = 2
        return true
    end
    
    -- Teleport
    if m:find('cormaya') or m:find('back') or m:find('passage') or m:find('pemaret') then
        npcHandler:say('Should I teleport you back to Pemaret?', cid)
        npcHandler.topic[cid] = 4  -- CHANGED FROM 3 TO 4
        return true
    end
    
    -- Spells
    if m == 'spells' then
        npcHandler:say('I can teach you {support spells}.', cid)
        return true
    end
    
    if m:find('support') then
        local available = {}
        for _, spell in ipairs(spells) do
            if not player:hasLearnedSpell(spell.name) and hasVocation(player, spell.vocations) then
                table.insert(available, spell.name)
            end
        end
        
        if #available > 0 then
            npcHandler:say("In this category I have " .. table.concat(available, ", ") .. ".", cid)
        else
            npcHandler:say("You know all my support spells.", cid)
        end
        return true
    end
    
    return false
end

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)

-- Register spell keywords
for _, spell in ipairs(spells) do
    keywordHandler:addKeyword(spell.keywords, function(cid) 
        return offerSpell(cid, spell.name, spell.price, spell.level, spell.vocations) 
    end)
end

-- Basic info keywords
keywordHandler:addKeyword({'pilgrimage'}, StdModule.say, {
    npcHandler = npcHandler, 
    text = 'Whenever you receive a lethal wound, your vital force is damaged and there is a chance that you lose some of your equipment. With every single of the five {blessings} you have, this damage and chance of loss will be reduced.'
})

keywordHandler:addKeyword({'blessings'}, StdModule.say, {
    npcHandler = npcHandler, 
    text = 'There are five blessings available in five sacred places: the {spiritual} shielding, the spark of the {phoenix}, the {embrace} of Tibia, the fire of the {suns} and the wisdom of {solitude}. Additionally, you can receive the {twist of fate} here.'
})

-- Blessing location info
keywordHandler:addKeyword({'spiritual', 'shield'}, StdModule.say, {
    npcHandler = npcHandler, 
    text = 'You can ask for the blessing of spiritual shielding in the whiteflower temple south of Thais.'
})

keywordHandler:addKeyword({'embrace'}, StdModule.say, {
    npcHandler = npcHandler, 
    text = 'The druids north of Carlin will provide you with the embrace of Tibia.'
})

keywordHandler:addKeyword({'suns', 'fire'}, StdModule.say, {
    npcHandler = npcHandler, 
    text = 'You can ask for the blessing of the two suns in the suntower near Ab\'Dendriel.'
})

keywordHandler:addKeyword({'phoenix', 'spark'}, StdModule.say, {
    npcHandler = npcHandler, 
    text = 'The spark of the phoenix is given by the dwarven priests of earth and fire in Kazordoon.'
})

keywordHandler:addKeyword({'job'}, StdModule.say, {
    npcHandler = npcHandler, 
    text = 'I teach some spells, provide one of the five blessings, and sell some amulets. Ask me for a trade if you like.'
})

keywordHandler:addKeyword({'name'}, StdModule.say, {
    npcHandler = npcHandler, 
    text = 'I am Eremo, an old man who has seen many things.'
})

npcHandler:setMessage(MESSAGE_GREET, 'Welcome to my little garden, adventurer |PLAYERNAME|!')
npcHandler:setMessage(MESSAGE_FAREWELL, 'It was a pleasure to help you, |PLAYERNAME|.')
npcHandler:setMessage(MESSAGE_WALKAWAY, 'It was a pleasure to help you, |PLAYERNAME|.')


-- Shop items (from XML parameters)
local shopItems = {
    {id = 2173, buy = 0, sell = 5000, subType = 0, name = "amulet of loss"},
    {id = 2196, buy = 0, sell = 50000, subType = 0, name = "broken amulet"},
    {id = 2173, buy = 10000, sell = 0, subType = 0, name = "amulet of loss"},
    {id = 2200, buy = 700, sell = 0, subType = 250, name = "protection amulet"},
}

-- Helper function to find shop item by id and subType (for fluid containers)
local function getShopItem(itemId, subType, isBuying)
    local itemType = ItemType(itemId)
    if itemType:isFluidContainer() then
        for _, item in ipairs(shopItems) do
            if item.id == itemId and item.subType == subType then
                return item
            end
        end
    end
    -- For non-fluid items, find the entry that matches the operation
    for _, item in ipairs(shopItems) do
        if item.id == itemId then
            if isBuying and item.buy > 0 then
                return item
            elseif not isBuying and item.sell > 0 then
                return item
            end
        end
    end
    -- Fallback to first match
    for _, item in ipairs(shopItems) do
        if item.id == itemId then
            return item
        end
    end
    return nil
end

local function openTradeWindow(cid, message, keywords, parameters, node)
    if not npcHandler:isFocused(cid) then return false end
    local player = Player(cid)
    if not player then return false end
    local npc = Npc(getNpcCid())
    local shopList = {}
    for _, item in ipairs(shopItems) do
        table.insert(shopList, {id = item.id, buy = item.buy, sell = item.sell, subType = item.subType or 0, name = item.name})
    end
    npc:openShopWindow(player, shopList, function() return true end, function() return true end)
    npcHandler:say('Take all the time you need to browse my wares.', cid)
    return true
end
keywordHandler:addKeyword({'trade'}, openTradeWindow, {npcHandler = npcHandler})


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

npcType:eventType(NPCS_EVENT_BUYITEM)
npcType:onBuyItem(function(npc, player, itemId, subType, amount, ignoreCap, inBackpacks)
    local shopItem = getShopItem(itemId, subType, true)
    if not shopItem or shopItem.buy <= 0 then return false end
    local totalCost = amount * shopItem.buy
    if player:getTotalMoney() < totalCost then
        player:sendCancelMessage("You don't have enough money.")
        return false
    end
    local itemSubType = shopItem.subType or 1
    local bought = doNpcSellItem(player:getId(), itemId, amount, itemSubType, ignoreCap, inBackpacks, ITEM_BACKPACK)
    if bought == 0 then
        player:sendCancelMessage("You do not have enough capacity.")
        return false
    end
    player:removeTotalMoney(bought * shopItem.buy)
    player:sendTextMessage(MESSAGE_INFO_DESCR, "Bought " .. bought .. "x " .. shopItem.name .. " for " .. (bought * shopItem.buy) .. " gold.")
    return true
end)

npcType:eventType(NPCS_EVENT_SELLITEM)
npcType:onSellItem(function(npc, player, itemId, subType, amount, ignoreEquipped)
    local shopItem = getShopItem(itemId, subType, false)
    if not shopItem or shopItem.sell <= 0 then return false end
    local totalPrice = amount * shopItem.sell
    local itemName = shopItem.name or ItemType(itemId):getName()
    
    local itemSubType = -1
    if ItemType(itemId):isFluidContainer() then
        itemSubType = subType
    end
    
    if doPlayerSellItem(player:getId(), itemId, amount, totalPrice, itemSubType, ignoreEquipped) then
        player:sendTextMessage(MESSAGE_INFO_DESCR, "Sold " .. amount .. "x " .. shopItem.name .. " for " .. (amount * shopItem.sell) .. " gold.")
        return true
    end
    player:sendCancelMessage("You do not have this object.")
    return false
end)

npcHandler:addModule(FocusModule:new())
npcType:register()