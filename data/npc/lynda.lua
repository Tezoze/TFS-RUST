-- Lynda - Converted from XML to Lua NpcType
-- Original XML: data/npc/Lynda.xml
-- Original Script: data/npc/scripts/Lynda.lua

local npcName = "Lynda"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a lynda")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 138, lookHead = 79, lookBody = 81, lookLegs = 67, lookFeet = 95})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local WAND_STORAGE = Storage.OutfitQuest.MageSummoner.AddonWand
local TIMER_STORAGE = Storage.OutfitQuest.MageSummoner.AddonWandTimer

-- ITEM CONFIGURATION
local REQUIRED_WANDS = {2181, 2182, 2183, 2185, 2186, 2187, 2188, 2189, 2190, 2191} -- All wands/rods
local MAGIC_SULPHUR = 5904
local SOUL_STONE = 5809
local ANKHS = 2193

local function creatureSayCallback(cid, type, msg)
    if not npcHandler:isFocused(cid) then
        return false
    end

    local player = Player(cid)
    local m = msg:lower()
    local storage = player:getStorageValue(WAND_STORAGE)

    -- 1. START QUEST (Angelina Trigger)
    if m:find("angelina") then
        if storage == 1 then
            npcHandler:say({
                "Angelina had been imprisoned? My, these are horrible news, but I am so glad to hear that she is safe now. ...",
                "I will happily carry out her wish and reward you, but I fear I need some important ingredients for my blessing spell first. ...",
                "Will you gather them for me?"
            }, cid)
            npcHandler.topic[cid] = 1
        end
        return true
    end

    -- 2. WANDS AND RODS HANDOFF
    if m:find("wand") or m:find("rod") then
        if storage == 2 then
            npcHandler:say("Did you bring a sample of each wand and each rod with you?", cid)
            npcHandler.topic[cid] = 3
        end
        return true
    end

    -- 3. MAGIC SULPHUR HANDOFF
    if m:find("sulphur") then
        if storage == 3 then
            npcHandler:say("Did you obtain 10 ounces of magic sulphur?", cid)
            npcHandler.topic[cid] = 4
        end
        return true
    end

    -- 4. SOUL STONE HANDOFF
    if m:find("soul stone") then
        if storage == 4 then
            npcHandler:say("Were you actually able to retrieve the Necromancer's soul stone?", cid)
            npcHandler.topic[cid] = 5
        end
        return true
    end

    -- 5. ANKHS HANDOFF
    if m:find("ankh") then
        if storage == 5 then
            npcHandler:say("Am I sensing enough holy energy from ankhs here?", cid)
            npcHandler.topic[cid] = 6
        end
        return true
    end

    -- 6. RITUAL / REWARD
    if m == "ritual" then
        if storage == 6 then
            local timerValue = player:getStorageValue(TIMER_STORAGE) or 0
            if timerValue < os.time() then
                npcHandler:say('I\'m glad to tell you that I have finished the ritual, player. Here is your new wand. I hope you carry it proudly for everyone to see.', cid)
                player:setStorageValue(WAND_STORAGE, 7)
                player:addOutfitAddon(141, 1)
                player:addOutfitAddon(130, 1)
                player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
            else
                npcHandler:say('Please let me focus for a while, |PLAYERNAME|. The ritual is not complete.', cid)
            end
        end
        return true
    end

    -- CONFIRMATION HANDLERS
    if m == "yes" then
        -- Accept Quest
        if npcHandler.topic[cid] == 1 then
            npcHandler:say({
                "Thank you, I promise that your efforts won't be in vain! Listen closely now: First, I need a sample of five druid rods and five sorcerer wands. ...",
                "I need a snakebite rod, a moonlight rod, a necrotic rod, a terra rod and a hailstorm rod. Then, I need a wand of vortex, a wand of dragonbreath ...",
                "... a wand of decay, a wand of cosmic energy and a wand of inferno. Please bring them all at once so that their energy will be balanced. ...",
                "Secondly, I need 10 ounces of magic sulphur. It can absorb the elemental energy of all the wands and rods and bind it to something else. ...",
                "Next, I will need a soul stone. These can be used as a vessel for energy, evil as well as good. They are rarely used nowaday though. ...",
                "Lastly, I need a lot of holy energy. I can extract it from ankhs, but only a small amount each time. I will need about 20 ankhs. ...",
                "Did you understand everything I told you and will help me with my blessing?"
            }, cid)
            npcHandler.topic[cid] = 2
            return true
        
        -- Confirm Understanding
        elseif npcHandler.topic[cid] == 2 then
            npcHandler:say("Alright then. Come back to with a sample of all five wands and five rods, please.", cid)
            player:setStorageValue(WAND_STORAGE, 2)
            npcHandler.topic[cid] = 0
            return true

        -- Give Wands/Rods
        elseif npcHandler.topic[cid] == 3 then
            local hasAll = true
            for _, itemId in ipairs(REQUIRED_WANDS) do
                if player:getItemCount(itemId) < 1 then
                    hasAll = false
                    break
                end
            end

            if hasAll then
                npcHandler:say("Thank you, that must have been a lot to carry. Now, please bring me 10 ounces of magic sulphur.", cid)
                for _, itemId in ipairs(REQUIRED_WANDS) do
                    player:removeItem(itemId, 1)
                end
                player:setStorageValue(WAND_STORAGE, 3)
            else
                npcHandler:say("You do not have all the required wands and rods.", cid)
            end
            npcHandler.topic[cid] = 0
            return true

        -- Give Sulphur
        elseif npcHandler.topic[cid] == 4 then
            if player:removeItem(MAGIC_SULPHUR, 10) then
                npcHandler:say("Very good. I will immediately start to prepare the ritual and extract the elemental energy from the wands and rods. Please bring me the Necromancer's soul stone now.", cid)
                player:setStorageValue(WAND_STORAGE, 4)
            else
                npcHandler:say("You do not have 10 ounces of magic sulphur.", cid)
            end
            npcHandler.topic[cid] = 0
            return true

        -- Give Soul Stone
        elseif npcHandler.topic[cid] == 5 then
            if player:removeItem(SOUL_STONE, 1) then
                npcHandler:say("You have found a rarity there, |PLAYERNAME|. This will become the tip of your blessed wand. Please bring me 20 ankhs now to complete the ritual.", cid)
                player:setStorageValue(WAND_STORAGE, 5)
            else
                npcHandler:say("You do not have the soul stone.", cid)
            end
            npcHandler.topic[cid] = 0
            return true

        -- Give Ankhs
        elseif npcHandler.topic[cid] == 6 then
            if player:removeItem(ANKHS, 20) then
                npcHandler:say("The ingredients for the ritual are complete! I will start to prepare your blessed wand, but I have to medidate first. Please come back later to hear how the ritual went.", cid)
                player:setStorageValue(WAND_STORAGE, 6)
                player:setStorageValue(TIMER_STORAGE, os.time() + 10800) -- 3 Hours
            else
                npcHandler:say("You do not have 20 ankhs.", cid)
            end
            npcHandler.topic[cid] = 0
            return true
        end
    end
    return true
end

-- ============================================================================
-- MARRIAGE SYSTEM LOGIC
-- ============================================================================

-- Function to check engagement (Missing from your original script, added here)
local function tryEngage(cid, message, keywords, parameters, node)
    if not npcHandler:isFocused(cid) then return false end
    
    local player = Player(cid)
    local candidateName = message
    local candidate = Player(candidateName)
    
    if not candidate then
        npcHandler:say('A player with this name is not online.', cid)
        return true
    end
    
    if player:getName() == candidate:getName() then
        npcHandler:say('You cannot marry yourself, narcissist.', cid)
        return true
    end
    
    if getPlayerMarriageStatus(player:getGuid()) ~= 0 then
        npcHandler:say('You are already married or engaged.', cid)
        return true
    end
    
    if getPlayerMarriageStatus(candidate:getGuid()) ~= 0 then
        npcHandler:say('Your partner is already married or engaged.', cid)
        return true
    end

    if player:getItemCount(ITEM_WEDDING_RING) < 1 then
        npcHandler:say('You need a wedding ring to propose.', cid)
        return true
    end
    
    -- Set status to Proposed
    setPlayerMarriageStatus(player:getGuid(), PROPOSED_STATUS)
    setPlayerSpouse(player:getGuid(), candidate:getGuid())
    player:removeItem(ITEM_WEDDING_RING, 1)
    
    npcHandler:say('You have proposed to ' .. candidate:getName() .. '. They must talk to me to accept.', cid)
    return true
end

local function confirmWedding(cid, message, keywords, parameters, node)
    if not npcHandler:isFocused(cid) then return false end

    local player = Player(cid)
    local playerStatus = getPlayerMarriageStatus(player:getGuid())
    local candidateGuid = getPlayerSpouse(player:getGuid())
    
    if playerStatus == PROPACCEPT_STATUS then
        local candidateName = getPlayerNameById(candidateGuid)
        
        -- Wedding Ceremony
        setPlayerMarriageStatus(player:getGuid(), MARRIED_STATUS)
        setPlayerMarriageStatus(candidateGuid, MARRIED_STATUS)
        
        delayedSay('Dear friends and family, we are gathered here today to witness and celebrate the union of ' .. candidateName .. ' and ' .. player:getName() .. ' in marriage.')
        delayedSay('Through their time together, they have come to realize that their personal dreams, hopes, and goals are more attainable and more meaningful through the combined effort and mutual support provided in love, commitment, and family;', 5000)
        delayedSay('and so they have decided to live together as husband and wife. And now, by the power vested in me by the Gods of Tibia, I hereby pronounce you husband and wife.', 15000)
        delayedSay('*After a whispered blessing opens an hand towards ' .. player:getName() .. '* Take these two engraved wedding rings and give one of them to your spouse.', 22000)
        delayedSay('You may now kiss your bride.', 28000)
        
        local item1 = player:addItem(ITEM_ENGRAVED_WEDDING_RING, 1)
        local item2 = player:addItem(ITEM_ENGRAVED_WEDDING_RING, 1)
        local dateStr = os.date('%B %d, %Y.')
        
        if item1 then item1:setAttribute(ITEM_ATTRIBUTE_DESCRIPTION, player:getName() .. ' & ' .. candidateName .. ' forever - married on ' .. dateStr) end
        if item2 then item2:setAttribute(ITEM_ATTRIBUTE_DESCRIPTION, player:getName() .. ' & ' .. candidateName .. ' forever - married on ' .. dateStr) end
    else
        npcHandler:say('Your partner didn\'t accept your proposal yet.', cid)
    end
    return true
end

local function confirmRemoveEngage(cid, message, keywords, parameters, node)
    if not npcHandler:isFocused(cid) then return false end

    local player = Player(cid)
    local playerStatus = getPlayerMarriageStatus(player:getGuid())
    local playerSpouse = getPlayerSpouse(player:getGuid())
    
    if playerStatus == PROPOSED_STATUS then
        npcHandler:say('Are you sure you want to remove your wedding proposal with {' .. getPlayerNameById(playerSpouse) .. '}?', cid)
        node:addChildKeyword({'no'}, StdModule.say, {npcHandler = npcHandler, onlyFocus = true, moveup = 3, text = 'Ok, let\'s keep it then.'})

        local function removeEngage(cid, message, keywords, parameters, node)
            player:addItem(ITEM_WEDDING_RING, 1)
            player:addItem(10503, 1) -- Wedding Box/Ticket return
            setPlayerMarriageStatus(player:getGuid(), 0)
            setPlayerSpouse(player:getGuid(), -1)
            npcHandler:say(parameters.text, cid)
            keywordHandler:moveUp(parameters.moveup)
        end
        node:addChildKeyword({'yes'}, removeEngage, {moveup = 3, text = 'Ok, your marriage proposal to {' .. getPlayerNameById(playerSpouse) .. '} has been removed. Take your wedding ring back.'})
    else
        npcHandler:say('You don\'t have any pending proposal to be removed.', cid)
        keywordHandler:moveUp(2)
    end
    return true
end

local function confirmDivorce(cid, message, keywords, parameters, node)
    if not npcHandler:isFocused(cid) then return false end

    local player = Player(cid)
    local playerStatus = getPlayerMarriageStatus(player:getGuid())
    local playerSpouse = getPlayerSpouse(player:getGuid())
    
    if playerStatus == MARRIED_STATUS then
        npcHandler:say('Are you sure you want to divorce of {' .. getPlayerNameById(playerSpouse) .. '}?', cid)
        node:addChildKeyword({'no'}, StdModule.say, {npcHandler = npcHandler, onlyFocus = true, moveup = 3, text = 'Great! Marriages should be an eternal commitment.'})

        local function divorce(cid, message, keywords, parameters, node)
            local player = Player(cid)
            local spouseGuid = getPlayerSpouse(player:getGuid())
            
            setPlayerMarriageStatus(player:getGuid(), 0)
            setPlayerSpouse(player:getGuid(), -1)
            
            -- Set spouse status (offline compatible)
            setPlayerMarriageStatus(spouseGuid, 0)
            setPlayerSpouse(spouseGuid, -1)
            
            npcHandler:say(parameters.text, cid)
            keywordHandler:moveUp(parameters.moveup)
        end
        node:addChildKeyword({'yes'}, divorce, {moveup = 3, text = 'Ok, you are now divorced of {' .. getPlayerNameById(playerSpouse) .. '}. Think better next time after marrying someone.'})
    else
        npcHandler:say('You aren\'t married to get a divorce.', cid)
        keywordHandler:moveUp(2)
    end
    return true
end

-- KEYWORD HANDLER MAPPINGS (Marriage)
local node1 = keywordHandler:addKeyword({'marry'}, StdModule.say, {npcHandler = npcHandler, onlyFocus = true, text = 'Would you like to get married? Make sure you have a wedding ring and the wedding outfit box with you.'})
node1:addChildKeyword({'no'}, StdModule.say, {npcHandler = npcHandler, onlyFocus = true, moveup = 1, text = 'That\'s fine.'})

local node2 = node1:addChildKeyword({'yes'}, StdModule.say, {npcHandler = npcHandler, onlyFocus = true, text = 'And who would you like to marry?'})
node2:addChildKeyword({'[%w]'}, tryEngage, {})

local node3 = keywordHandler:addKeyword({'celebration'}, StdModule.say, {npcHandler = npcHandler, onlyFocus = true, text = 'Is your soulmate and friends here with you for the celebration?.'})
node3:addChildKeyword({'no'}, StdModule.say, {npcHandler = npcHandler, onlyFocus = true, moveup = 1, text = 'Then go bring them here!.'})

local node4 = node3:addChildKeyword({'yes'}, StdModule.say, {npcHandler = npcHandler, onlyFocus = true, text = 'Good, let\'s {begin} then!.'}) 
node4:addChildKeyword({'begin'}, confirmWedding, {})

keywordHandler:addKeyword({'remove'}, confirmRemoveEngage, {})
keywordHandler:addKeyword({'divorce'}, confirmDivorce, {})

npcHandler:setMessage(MESSAGE_GREET, "Welcome in the name of the gods, pilgrim |PLAYERNAME|!")
npcHandler:setMessage(MESSAGE_FAREWELL, "Be careful on your journeys.")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Be careful on your journeys.")

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
