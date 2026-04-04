-- Quentin - Converted from XML to Lua NpcType
-- Original XML: data/npc/Quentin.xml
-- Original Script: data/npc/scripts/Quentin.lua

local npcName = "Quentin"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a quentin")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 57})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


-- Main callback
local function creatureSayCallback(cid, type, msg)
    if not npcHandler:isFocused(cid) then
        return false
    end
    
    local player = Player(cid)
    if not player then
        return false
    end

    local pvpBlessCost = StdModule.calculateTwistOfFateBlessingCost(player:getLevel())

    -- HEALING LOGIC
    if msgcontains(msg, "heal") then
        local healed = false
        
        -- Condition healing
        if player:getCondition(CONDITION_FIRE) then
            npcHandler:say("You are burning. Let me quench those flames.", cid)
            player:removeCondition(CONDITION_FIRE)
            player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
            healed = true
        elseif player:getCondition(CONDITION_POISON) then
            npcHandler:say("You are poisoned. Let me soothe your pain.", cid)
            player:removeCondition(CONDITION_POISON)
            player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
            healed = true
        elseif player:getCondition(CONDITION_ENERGY) then
            npcHandler:say("You are electrified, my child. Let me help you to stop trembling.", cid)
            player:removeCondition(CONDITION_ENERGY)
            player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
            healed = true
        elseif player:getCondition(CONDITION_PARALYZE) then
            npcHandler:say("You are paralyzed. Let me cure you.", cid)
            player:removeCondition(CONDITION_PARALYZE)
            player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
            healed = true
        elseif player:getCondition(CONDITION_DROWN) then
            npcHandler:say("You are drowning. Let me help you.", cid)
            player:removeCondition(CONDITION_DROWN)
            player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
            healed = true
        elseif player:getCondition(CONDITION_FREEZING) then
            npcHandler:say("You are freezing! Let me warm you up.", cid)
            player:removeCondition(CONDITION_FREEZING)
            player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
            healed = true
        elseif player:getCondition(CONDITION_BLEEDING) then
            npcHandler:say("You are bleeding! Let me stop that.", cid)
            player:removeCondition(CONDITION_BLEEDING)
            player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
            healed = true
        elseif player:getCondition(CONDITION_DAZZLED) then
            npcHandler:say("You are dazzled! Do not mess with holy creatures anymore!", cid)
            player:removeCondition(CONDITION_DAZZLED)
            player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
            healed = true
        elseif player:getCondition(CONDITION_CURSED) then
            npcHandler:say("You are cursed! I will remove it.", cid)
            player:removeCondition(CONDITION_CURSED)
            player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
            healed = true
            
        -- HP Healing (Strictly < 65 HP only)
        elseif player:getHealth() < 65 then
            npcHandler:say("You are looking really bad. Let me heal your wounds.", cid)
            local health = player:getHealth()
            if health < 65 then
                player:addHealth(65 - health)
            end
            player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
            healed = true
        end
        
        if not healed then
            npcHandler:say("You aren't looking that bad. Sorry, I can't help you.", cid)
        end
        return true

    -- WOODEN STAKE QUEST
    elseif msgcontains(msg, "stake") then
        local stakeStorage = player:getStorageValue(Storage.FriendsAndTraders.TheBlessedStake)
        
        if stakeStorage == -1 then
            npcHandler:say({
                'A blessed stake to defeat evil spirits? I do know an old prayer which is said to grant sacred power and to be able to bind this power to someone, or something. ...',
                'However, this prayer needs the combined energy of ten priests. Each of them has to say one line of the prayer. ...',
                'I could start with the prayer, but since the next priest has to be in a different location, you probably will have to travel a lot. ...',
                'Is this stake really important enough to you so that you are willing to take this burden?'
            }, cid)
            npcHandler.topic[cid] = 1
        elseif stakeStorage == 1 then
            if player:getItemCount(5941) == 0 then
                npcHandler:say('I guess you couldn\'t convince Gamon to give you a stake, eh?', cid)
            else
                npcHandler:say('Yes, I was informed what to do. Are you prepared to receive my line of the prayer?', cid)
                npcHandler.topic[cid] = 2
            end
        elseif stakeStorage == 2 then
            npcHandler:say('You should visit Tibra in the Carlin church now.', cid)
        elseif stakeStorage > 2 then
            npcHandler:say('You already received my line of the prayer.', cid)
        end
        return true

    -- TWIST OF FATE BLESSING
    elseif msgcontains(msg, "twist") or msgcontains(msg, "fate") then
        npcHandler:say({
            'This is a special blessing I can bestow upon you once you have obtained at least one of the other blessings and which functions a bit differently. ...',
            'It only works when you\'re killed by other adventurers, which means that at least forty percent of the damage leading to your death was caused by others, not by monsters or the environment. ...',
            'The twist of fate will not reduce the death penalty like the other blessings, but instead prevent you from losing your other blessings as well as the amulet of loss, should you wear one. It costs the same as the other blessings. ...',
            'Would you like to receive that protection for a sacrifice of ' .. pvpBlessCost .. ' gold, child?'
        }, cid)
        npcHandler.topic[cid] = 3
        return true

    -- CONFIRMATIONS
    elseif msgcontains(msg, "yes") then
        if npcHandler.topic[cid] == 1 then -- Accept Stake Quest
            npcHandler:say('Alright, I guess you need a stake first. Maybe Gamon can help you, the leg of a chair or something could just do. Try asking him for a stake, and if you have one, bring it back to me.', cid)
            player:setStorageValue(Storage.FriendsAndTraders.Questline, 1)
            player:setStorageValue(Storage.FriendsAndTraders.TheBlessedStake, 1)
            npcHandler.topic[cid] = 0
            return true
        elseif npcHandler.topic[cid] == 2 then -- Bless Stake
            npcHandler:say('So receive my prayer: \'Light shall be near - and darkness afar\'. Now, bring your stake to Tibra in the Carlin church for the next line of the prayer. I will inform her what to do.', cid)
            player:setStorageValue(Storage.FriendsAndTraders.TheBlessedStake, 2)
            player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
            npcHandler.topic[cid] = 0
            return true
        elseif npcHandler.topic[cid] == 3 then -- Buy Twist of Fate
            if player:hasBlessing(6) then
                npcHandler:say("Gods have already blessed you with this blessing!", cid)
            elseif not player:removeTotalMoney(pvpBlessCost) then
                npcHandler:say("You don't have enough money for blessing.", cid)
            else
                player:addBlessing(6)
                npcHandler:say("So receive the protection of the twist of fate, pilgrim.", cid)
                player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
            end
            npcHandler.topic[cid] = 0
            return true
        end
    elseif msgcontains(msg, "no") then
        if npcHandler.topic[cid] > 0 then
            npcHandler:say('I will wait for you.', cid)
            npcHandler.topic[cid] = 0
            return true
        end
    end
    
    return false
end

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)

npcHandler:setMessage(MESSAGE_GREET, 'Welcome, adventurer |PLAYERNAME|! If you are new in {Tibia}, ask me for {help} or {healing}.')
npcHandler:setMessage(MESSAGE_WALKAWAY, 'Remember: If you are heavily wounded or suffering from conditions, I can heal you for free.')
npcHandler:setMessage(MESSAGE_FAREWELL, 'Good bye, |PLAYERNAME|!')

-- Tibia and basic keywords
keywordHandler:addKeyword({'tibia'}, StdModule.say, {npcHandler = npcHandler, text = 'That is where we are. The world of Tibia. Admire its beauty.'})
keywordHandler:addKeyword({'help'}, StdModule.say, {npcHandler = npcHandler, text = 'First you should try to get some gold to buy better equipment.'})
keywordHandler:addKeyword({'quest'}, StdModule.say, {npcHandler = npcHandler, text = 'First you should try to get some gold to buy better equipment.'})
keywordHandler:addKeyword({'task'}, StdModule.say, {npcHandler = npcHandler, text = 'First you should try to get some gold to buy better equipment.'})

-- Personal information
keywordHandler:addKeyword({'job'}, StdModule.say, {npcHandler = npcHandler, text = 'Job? I have no job. I just live for the gods of Tibia.'})
keywordHandler:addKeyword({'equipment'}, StdModule.say, {npcHandler = npcHandler, text = 'First you should buy a bag or backpack. That way your hands will be free to hold a weapon and a shield.'})
keywordHandler:addKeyword({'name'}, StdModule.say, {npcHandler = npcHandler, text = 'My name is Quentin.'})

-- Kings and military
keywordHandler:addKeyword({'king'}, StdModule.say, {npcHandler = npcHandler, text = 'Our [king] resides in [the castle] to the west.'})
keywordHandler:addKeyword({'general'}, StdModule.say, {npcHandler = npcHandler, text = 'Harkath Bloodblade is his name.'})
keywordHandler:addKeyword({'army'}, StdModule.say, {npcHandler = npcHandler, text = 'I don\'t know much about the Tibian army. Ask general Harkath Bloodblade about that.'})

-- Gods and life
keywordHandler:addKeyword({'gods'}, StdModule.say, {npcHandler = npcHandler, text = 'They created Tibia and all life on it.'})
keywordHandler:addKeyword({'life'}, StdModule.say, {npcHandler = npcHandler, text = 'On Tibia there are many forms of life. There are plants and people and monsters.'})
keywordHandler:addKeyword({'plants'}, StdModule.say, {npcHandler = npcHandler, text = 'Just walk around, you will see grass, trees, and bushes.'})
keywordHandler:addKeyword({'monsters'}, StdModule.say, {npcHandler = npcHandler, text = 'There are really too many of them in Tibia. But who am I to challenge the wisdom of the gods?'})
keywordHandler:addKeyword({'people'}, StdModule.say, {npcHandler = npcHandler, text = 'I am a simple monk. I just know Sam, Frodo, and Gorn. They all live in the main street to the north.'})

-- Time and money
keywordHandler:addKeyword({'time'}, StdModule.say, {npcHandler = npcHandler, text = 'Now, it is 5:52 pm. Ask [Gorn] for a watch, if you need one.'})
keywordHandler:addKeyword({'money'}, StdModule.say, {npcHandler = npcHandler, text = 'If you need money you should slay monsters and take their gold. Look for spiders and rats.'})
keywordHandler:addKeyword({'gold'}, StdModule.say, {npcHandler = npcHandler, text = 'If you need money you should slay monsters and take their gold. Look for spiders and rats.'})

-- Locations and creatures
keywordHandler:addKeyword({'rats'}, StdModule.say, {npcHandler = npcHandler, text = 'There are sewers underneath the city. They say these sewers are brimming with rats.'})
keywordHandler:addKeyword({'sewers'}, StdModule.say, {npcHandler = npcHandler, text = 'You can enter the sewers through a sewer grate. But watch out. There are many rats. And don\'t forget to bring a torch.'})
keywordHandler:addKeyword({'spiders'}, StdModule.say, {npcHandler = npcHandler, text = 'There are spider\'s nests beyond our city near Gorn\'s shop and at the McRonalds\' farm in the east.'})

-- News and food
keywordHandler:addKeyword({'news'}, StdModule.say, {npcHandler = npcHandler, text = 'Sorry, I know nothing new. Please ask Frodo about that topic.'})
keywordHandler:addKeyword({'food'}, StdModule.say, {npcHandler = npcHandler, text = 'If you would like to heal your wounds you should eat some food. Frodo sells excellent meals. But if you are very weak you can also come to me. I will heal you.'})

-- NPCs
keywordHandler:addKeyword({'ferumbras'}, StdModule.say, {npcHandler = npcHandler, text = 'Hush! Do not mention the Evil One in these walls.'})
keywordHandler:addKeyword({'baxter'}, StdModule.say, {npcHandler = npcHandler, text = 'He is the guard of the royal castle.'})
keywordHandler:addKeyword({'bozo'}, StdModule.say, {npcHandler = npcHandler, text = 'He is the king\'s jester, but he believes himself to be the king of fools.'})
keywordHandler:addKeyword({'eclesius'}, StdModule.say, {npcHandler = npcHandler, text = 'I hope I won\'t become that forgetful when I\'m old. But he is a good man, although I don\'t know him very well'})
keywordHandler:addKeyword({'elane'}, StdModule.say, {npcHandler = npcHandler, text = 'She is the leader of the local Paladins\' guild.'})
keywordHandler:addKeyword({'frodo'}, StdModule.say, {npcHandler = npcHandler, text = 'He is the owner of Frodo\'s Hut, the tavern north of this temple.'})
keywordHandler:addKeyword({'gorn'}, StdModule.say, {npcHandler = npcHandler, text = 'He is selling equipment. If you still have no backpack you should go and ask him for one.'})
keywordHandler:addKeyword({'gregor'}, StdModule.say, {npcHandler = npcHandler, text = 'The leader of the Knights\' guild is a man of few words.'})
keywordHandler:addKeyword({'harkath bloodblade'}, StdModule.say, {npcHandler = npcHandler, text = 'A hard man but his heart is in the right place.'})
keywordHandler:addKeyword({'lugri'}, StdModule.say, {npcHandler = npcHandler, text = 'Please do not mention the fallen one.'})
keywordHandler:addKeyword({'lynda'}, StdModule.say, {npcHandler = npcHandler, text = 'She is a highly competent priest.'})
keywordHandler:addKeyword({'marvik'}, StdModule.say, {npcHandler = npcHandler, text = 'I admire the healing skills of Marvik.'})
keywordHandler:addKeyword({'mcronald'}, StdModule.say, {npcHandler = npcHandler, text = 'The McRonalds run the local farm.'})
keywordHandler:addKeyword({'muriel'}, StdModule.say, {npcHandler = npcHandler, text = 'Muriel is a famous sorcerer. She is the keeper of arcane secrets that are known only to few mortals.'})
keywordHandler:addKeyword({'oswald'}, StdModule.say, {npcHandler = npcHandler, text = 'This man is spreading horrible rumours all the time.'})
keywordHandler:addKeyword({'sam'}, StdModule.say, {npcHandler = npcHandler, text = 'He is our blacksmith. He sells weapons and armour.'})


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
