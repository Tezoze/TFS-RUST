-- King Tibianus - Converted from XML to Lua NpcType
-- Original XML: data/npc/King Tibianus.xml
-- Original Script: data/npc/scripts/King Tibianus.lua

local npcName = "King Tibianus"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a king tibianus")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 332})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


-- STORAGE CONSTANTS
local FRONTIER_QUEST = Storage.TheNewFrontier.Questline
local FRONTIER_BRIBE = Storage.TheNewFrontier.BribeKing
local FRONTIER_MISSION = Storage.TheNewFrontier.Mission05

local GOLDEN_OUTFIT = Storage.OutfitQuest.GoldenBaseOutfit
local GOLDEN_ADDON_1 = Storage.OutfitQuest.GoldenFirstAddon
local GOLDEN_ADDON_2 = Storage.OutfitQuest.GoldenSecondAddon

local function creatureSayCallback(cid, type, msg)
    if not npcHandler:isFocused(cid) then
        return false
    end

    local player = Player(cid)
    local m = msg:lower()

    -- ==========================================================
    -- THE NEW FRONTIER QUEST
    -- ==========================================================
    if m:find("farmine") then
        if player:getStorageValue(FRONTIER_QUEST) == 15 then
            npcHandler:say("Ah, I vaguely remember that our little allies were eager to build some base. So speak up, what do you want?", cid)
            npcHandler.topic[cid] = 1
        end
        return true
    end

    if m:find("flatter") then
        if npcHandler.topic[cid] == 1 then
            if player:getStorageValue(FRONTIER_BRIBE) < 1 then
                npcHandler:say("The idea of a promising market and new resources suits us quite well. I think it is reasonable to send some assistance.", cid)
                player:setStorageValue(FRONTIER_BRIBE, 1)
                player:setStorageValue(FRONTIER_MISSION, player:getStorageValue(FRONTIER_MISSION) + 1)
            end
            npcHandler.topic[cid] = 0
        end
        return true
    end

    -- ==========================================================
    -- GOLDEN OUTFIT QUEST
    -- ==========================================================
    if m == "outfit" or m == "addon" then
        local hasBase = player:getStorageValue(GOLDEN_OUTFIT) >= 1
        local hasAddon1 = player:getStorageValue(GOLDEN_ADDON_1) >= 1
        local hasAddon2 = player:getStorageValue(GOLDEN_ADDON_2) >= 1

        if not hasBase then
            npcHandler:say("In exchange for a truly generous donation, I will offer a special outfit. Do you want to make a donation?", cid)
            npcHandler.topic[cid] = 10 -- Topic 10: Base Outfit Offer
        elseif not hasAddon1 or not hasAddon2 then
            npcHandler:say("In exchange for a truly generous donation, I will offer a special outfit. Do you want to make a donation?", cid)
            npcHandler.topic[cid] = 11 -- Topic 11: Addon Offer
        else
            npcHandler:say("You already have all the golden attire I can provide.", cid)
        end
        return true
    end

    -- Explanation Logic
    if m == "yes" then
        if npcHandler.topic[cid] == 10 then
            npcHandler:say({
                "Excellent! Now, let me explain. If you donate 1.000.000 gold pieces, you will be entitled to wear a unique outfit. ...",
                "You will be entitled to wear the {armor} for 500.000 gold pieces, {boots} for an additional 250.000 and the {helmet} for another 250.000 gold pieces. ...",
                "What will it be?"
            }, cid)
            npcHandler.topic[cid] = 12 -- Topic 12: Waiting for "armor" selection
            return true
        elseif npcHandler.topic[cid] == 11 then
            npcHandler:say({
                "Excellent! Now, let me explain. If you donate 1.000.000 gold pieces, you will be entitled to wear a unique outfit. ...",
                "You will be entitled to wear the {armor} for 500.000 gold pieces, {boots} for an additional 250.000 and the {helmet} for another 250.000 gold pieces. ...",
                "What will it be?"
            }, cid)
            npcHandler.topic[cid] = 13 -- Topic 13: Waiting for "boots" or "helmet" selection
            return true
        end
    end

    -- Selection Logic
    if m == "armor" and npcHandler.topic[cid] == 12 then
        npcHandler:say("So you would like to donate 500.000 gold pieces which in return will entitle you to wear a unique armor?", cid)
        npcHandler.topic[cid] = 14 -- Topic 14: Confirm Armor
        return true
    end

    if m == "boots" and npcHandler.topic[cid] == 13 then
        if player:getStorageValue(GOLDEN_ADDON_1) < 1 then
            npcHandler:say("So you would like to donate 250.000 gold pieces which in return will entitle you to wear unique boots?", cid)
            npcHandler.topic[cid] = 15 -- Topic 15: Confirm Boots
        else
            npcHandler:say("You already have the golden boots addon.", cid)
        end
        return true
    end

    if m == "helmet" and npcHandler.topic[cid] == 13 then
        if player:getStorageValue(GOLDEN_ADDON_2) < 1 then
            npcHandler:say("So you would like to donate 250.000 gold pieces which in return will entitle you to wear a unique helmet?", cid)
            npcHandler.topic[cid] = 16 -- Topic 16: Confirm Helmet
        else
            npcHandler:say("You already have the golden helmet addon.", cid)
        end
        return true
    end

    -- Purchase Confirmation
    if m == "yes" then
        -- BUY ARMOR
        if npcHandler.topic[cid] == 14 then
            if player:getMoney() + player:getBankBalance() >= 500000 then
                player:removeMoneyNpc(500000)
                player:addOutfit(1211)
                player:addOutfit(1210)
                player:setStorageValue(GOLDEN_OUTFIT, 1)
                npcHandler:say("Take this armor as a token of great gratitude. Let us forever remember this day, my friend!", cid)
            else
                npcHandler:say("You do not have enough money to donate that amount.", cid)
            end
            npcHandler.topic[cid] = 0
            return true
        
        -- BUY BOOTS
        elseif npcHandler.topic[cid] == 15 then
            if player:getMoney() + player:getBankBalance() >= 250000 then
                player:removeMoneyNpc(250000)
                player:addOutfitAddon(1210, 2)
                player:addOutfitAddon(1211, 2)
                player:setStorageValue(GOLDEN_ADDON_1, 1)
                npcHandler:say("Take these boots as a token of great gratitude. Let us forever remember this day, my friend.", cid)
            else
                npcHandler:say("You do not have enough money to donate that amount.", cid)
            end
            npcHandler.topic[cid] = 0
            return true

        -- BUY HELMET
        elseif npcHandler.topic[cid] == 16 then
            if player:getMoney() + player:getBankBalance() >= 250000 then
                player:removeMoneyNpc(250000)
                player:addOutfitAddon(1210, 1)
                player:addOutfitAddon(1211, 1)
                player:setStorageValue(GOLDEN_ADDON_2, 1)
                npcHandler:say("Take this helmet as a token of great gratitude. Let us forever remember this day, my friend.", cid)
            else
                npcHandler:say("You do not have enough money to donate that amount.", cid)
            end
            npcHandler.topic[cid] = 0
            return true
        end
    end

    -- ==========================================================
    -- PROMOTION SYSTEM
    -- ==========================================================
    if m:find("promotion") or m:find("promote") then
        if player:getStorageValue(PlayerStorageKeys.promotion) == 1 then
            npcHandler:say("You are already promoted.", cid)
        elseif player:getLevel() < 20 then
            npcHandler:say("You need to be at least level 20 to be promoted.", cid)
        else
            npcHandler:say("I can promote you for 20000 gold coins. Do you want me to promote you?", cid)
            npcHandler.topic[cid] = 20
        end
        return true
    end

    if m == "yes" and npcHandler.topic[cid] == 20 then
        if player:getStorageValue(PlayerStorageKeys.promotion) == 1 then
             npcHandler:say("You are already promoted.", cid)
        elseif not player:removeTotalMoney(20000) then
            npcHandler:say("You do not have enough money.", cid)
        else
            local promotion = player:getVocation():getPromotion()
            player:setVocation(promotion)
            player:setStorageValue(PlayerStorageKeys.promotion, 1)
            npcHandler:say("Congratulations! You are now promoted.", cid)
        end
        npcHandler.topic[cid] = 0
        return true
    end

    return true
end

npcHandler:setMessage(MESSAGE_GREET, 'I greet thee, my loyal subject |PLAYERNAME|. How may I help you?')
npcHandler:setMessage(MESSAGE_FAREWELL, 'Good bye, |PLAYERNAME|!')
npcHandler:setMessage(MESSAGE_WALKAWAY, 'How rude!')

local function greetCallback(cid)
    npcHandler.topic[cid] = 0
    return true
end

npcHandler:setCallback(CALLBACK_GREET, greetCallback)
npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)

-- FLAVOR KEYWORDS
keywordHandler:addKeyword({'eremo'}, StdModule.say, {npcHandler = npcHandler, text = 'It is said that he lives on a small island near Edron. Maybe the people there know more about him.'})
keywordHandler:addKeyword({'job'}, StdModule.say, {npcHandler = npcHandler, text = 'I am your sovereign, King Tibianus III, and it\'s my duty to uphold {justice} and provide guidance for my subjects.'})
keywordHandler:addKeyword({'justice'}, StdModule.say, {npcHandler = npcHandler, text = 'I try my best to be just and fair to our citizens. The army and the {TBI} are a great help in fulfilling this duty.'})
keywordHandler:addKeyword({'name'}, StdModule.say, {npcHandler = npcHandler, text = 'Preposterous! You must know the name of your own King!'})
keywordHandler:addKeyword({'news'}, StdModule.say, {npcHandler = npcHandler, text = 'The latest news is usually brought to our magnificent town by brave adventurers. They recount tales of their journeys at Frodo\'s tavern.'})
keywordHandler:addKeyword({'how', 'are', 'you'}, StdModule.say, {npcHandler = npcHandler, text = 'Thank you, I\'m fine.'})
keywordHandler:addKeyword({'castle'}, StdModule.say, {npcHandler = npcHandler, text = 'Rain Castle is my home.'})
keywordHandler:addKeyword({'sell'}, StdModule.say, {npcHandler = npcHandler, text = 'Sell? Sell what? My kingdom isn\'t for sale!'})
keywordHandler:addKeyword({'god'}, StdModule.say, {npcHandler = npcHandler, text = 'Honour the Gods and above all pay your {taxes}.'})
keywordHandler:addKeyword({'zathroth'}, StdModule.say, {npcHandler = npcHandler, text = 'Please ask a priest about the gods.'})
keywordHandler:addKeyword({'citizen'}, StdModule.say, {npcHandler = npcHandler, text = 'The citizens of Tibia are my subjects. Ask the old monk Quentin if you want to learn more about them.'})
keywordHandler:addKeyword({'sam'}, StdModule.say, {npcHandler = npcHandler, text = 'He is a skilled blacksmith and a loyal subject.'})
keywordHandler:addKeyword({'frodo'}, StdModule.say, {npcHandler = npcHandler, text = 'He is the owner of Frodo\'s Hut and a faithful tax-payer.'})
keywordHandler:addKeyword({'gorn'}, StdModule.say, {npcHandler = npcHandler, text = 'He was once one of Tibia\'s greatest fighters. Now he sells equipment.'})
keywordHandler:addKeyword({'benjamin'}, StdModule.say, {npcHandler = npcHandler, text = 'He was once my greatest general. Now he is very old and senile so we assigned him to work for the Royal Tibia Mail.'})
keywordHandler:addKeyword({'noodles'}, StdModule.say, {npcHandler = npcHandler, text = 'The royal poodle Noodles is my greatest {treasure}!'})
keywordHandler:addKeyword({'ferumbras'}, StdModule.say, {npcHandler = npcHandler, text = 'He is a follower of the evil God Zathroth and responsible for many attacks on us. Kill him on sight!'})
keywordHandler:addKeyword({'bozo'}, StdModule.say, {npcHandler = npcHandler, text = 'He is my royal jester and cheers me up now and then.'})
keywordHandler:addKeyword({'treasure'}, StdModule.say, {npcHandler = npcHandler, text = 'The royal poodle Noodles is my greatest treasure!'})
keywordHandler:addKeyword({'monster'}, StdModule.say, {npcHandler = npcHandler, text = 'Go and hunt them! For king and country!'})
keywordHandler:addKeyword({'help'}, StdModule.say, {npcHandler = npcHandler, text = 'Visit Quentin the monk for help.'})
keywordHandler:addKeyword({'sewer'}, StdModule.say, {npcHandler = npcHandler, text = 'What a disgusting topic!'})
keywordHandler:addKeyword({'dungeon'}, StdModule.say, {npcHandler = npcHandler, text = 'Dungeons are no places for kings.'})
keywordHandler:addKeyword({'equipment'}, StdModule.say, {npcHandler = npcHandler, text = 'Feel free to buy it in our town\'s fine shops.'})
keywordHandler:addKeyword({'food'}, StdModule.say, {npcHandler = npcHandler, text = 'Ask the royal cook for some food.'})
keywordHandler:addKeyword({'tax collector'}, StdModule.say, {npcHandler = npcHandler, text = 'That tax collector is the bane of my life. He is so lazy. I bet you haven\'t payed any taxes at all.'})
keywordHandler:addKeyword({'king'}, StdModule.say, {npcHandler = npcHandler, text = 'I am the king, so watch what you say!'})
keywordHandler:addKeyword({'army'}, StdModule.say, {npcHandler = npcHandler, text = 'Ask the soldiers about that.'})
keywordHandler:addKeyword({'shop'}, StdModule.say, {npcHandler = npcHandler, text = 'Visit the shops of our merchants and craftsmen.'})
keywordHandler:addKeyword({'guild'}, StdModule.say, {npcHandler = npcHandler, text = 'The four major guilds are the knights, the paladins, the druids, and the sorcerers.'})
keywordHandler:addKeyword({'minotaur'}, StdModule.say, {npcHandler = npcHandler, text = 'Vile monsters, but I must admit they are strong and sometimes even cunning ... in their own bestial way.'})
keywordHandler:addKeyword({'good'}, StdModule.say, {npcHandler = npcHandler, text = 'The forces of good are hard pressed in these dark times.'})
keywordHandler:addKeyword({'evil'}, StdModule.say, {npcHandler = npcHandler, text = 'We need all strength we can muster to smite evil!'})
keywordHandler:addKeyword({'order'}, StdModule.say, {npcHandler = npcHandler, text = 'We need order to survive!'})
keywordHandler:addKeyword({'chaos'}, StdModule.say, {npcHandler = npcHandler, text = 'Chaos arises from selfishness.'})
keywordHandler:addKeyword({'excalibug'}, StdModule.say, {npcHandler = npcHandler, text = 'It\'s the sword of the Kings. If you return this weapon to me I will {reward} you beyond your wildest dreams.'})
keywordHandler:addKeyword({'reward'}, StdModule.say, {npcHandler = npcHandler, text = 'Well, if you want a reward, go on a quest to bring me Excalibug!'})
keywordHandler:addKeyword({'chester'}, StdModule.say, {npcHandler = npcHandler, text = 'A very competent person. A little nervous but very competent.'})
keywordHandler:addKeyword({'tbi'}, StdModule.say, {npcHandler = npcHandler, text = 'This organisation is an essential tool for holding our enemies in check. Its headquarter is located in the bastion in the northwall.'})
keywordHandler:addKeyword({'tibia'}, StdModule.say, {npcHandler = npcHandler, text = 'Soon the whole land will be ruled by me once again!'})
keywordHandler:addAliasKeyword({'land'})
keywordHandler:addKeyword({'harkath'}, StdModule.say, {npcHandler = npcHandler, text = 'Harkath Bloodblade is the general of our glorious {army}.'})
keywordHandler:addAliasKeyword({'bloodblade'})
keywordHandler:addAliasKeyword({'general'})
keywordHandler:addKeyword({'quest'}, StdModule.say, {npcHandler = npcHandler, text = 'I will call for heroes as soon as the need arises again and then reward them appropriately.'})
keywordHandler:addAliasKeyword({'mission'})
keywordHandler:addKeyword({'gold'}, StdModule.say, {npcHandler = npcHandler, text = 'To pay your taxes, visit the royal tax collector.'})
keywordHandler:addAliasKeyword({'money'})
keywordHandler:addAliasKeyword({'tax'})
keywordHandler:addAliasKeyword({'collector'})
keywordHandler:addKeyword({'time'}, StdModule.say, {npcHandler = npcHandler, text = 'It\'s a time for heroes!'})
keywordHandler:addAliasKeyword({'hero'})
keywordHandler:addAliasKeyword({'adventurer'})
keywordHandler:addKeyword({'enemy'}, StdModule.say, {npcHandler = npcHandler, text = 'Our enemies are numerous. The evil minotaurs, Ferumbras, and the renegade city of Carlin to the north are just some of them.'})
keywordHandler:addAliasKeyword({'enemies'})
keywordHandler:addKeyword({'carlin'}, StdModule.say, {npcHandler = npcHandler, text = 'They dare to reject my reign over the whole continent!'})
keywordHandler:addKeyword({'thais'}, StdModule.say, {npcHandler = npcHandler, text = 'Our beloved city has some fine shops, guildhouses and a modern sewerage system.'})
keywordHandler:addAliasKeyword({'city'})
keywordHandler:addKeyword({'merchant'}, StdModule.say, {npcHandler = npcHandler, text = 'Ask around about them.'})
keywordHandler:addAliasKeyword({'craftsmen'})
keywordHandler:addKeyword({'paladin'}, StdModule.say, {npcHandler = npcHandler, text = 'The paladins are great protectors for Thais.'})
keywordHandler:addAliasKeyword({'elane'})
keywordHandler:addKeyword({'knight'}, StdModule.say, {npcHandler = npcHandler, text = 'The brave knights are necessary for human survival in Thais.'})
keywordHandler:addAliasKeyword({'gregor'})
keywordHandler:addKeyword({'sorcerer'}, StdModule.say, {npcHandler = npcHandler, text = 'The magic of the sorcerers is a powerful tool to smite our enemies.'})
keywordHandler:addAliasKeyword({'muriel'})
keywordHandler:addKeyword({'druid'}, StdModule.say, {npcHandler = npcHandler, text = 'We need the druidic healing powers to fight evil.'})
keywordHandler:addAliasKeyword({'marvik'})

-- Custom Focus Module for "Hail King"
-- This overrides the default 'hi' behavior for this specific NPC instance
local focusModule = FocusModule:new()
focusModule:addGreetMessage('hail king')
focusModule:addGreetMessage('salutations king')
focusModule:addGreetMessage('hi')
focusModule:addGreetMessage('hello')
npcHandler:addModule(focusModule)


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

npcType:register()
