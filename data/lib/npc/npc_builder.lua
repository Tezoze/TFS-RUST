-- NpcBuilder: Declarative base class for builder-pattern NPC definitions
-- Provides chainable property setters, keyword registration, focus management,
-- event handlers, and the register() method that creates and configures NpcType.
-- All mutable per-spawn state lives in InstanceState keyed by npc:getId().
-- Builder instances are shared across all spawns of the same NPC name (immutable config only).

NpcBuilder = {}
NpcBuilder.__index = NpcBuilder

function NpcBuilder:new(name, outfit)
    local obj = {
        _name = name,
        _outfit = outfit,
        _health = 100,
        _maxHealth = 100,
        _walkInterval = 2000,
        _walkRadius = 2,
        _baseSpeed = 100,
        _floorChange = false,
        _pushable = false,
        _speechBubble = SPEECHBUBBLE_NORMAL,
        _talkRadius = 3,
        _idleTime = 120,
        _greetMsg = "Greetings, |PLAYERNAME|.",
        _farewellMsg = "Good bye, |PLAYERNAME|.",
        _walkawayMsg = "Good bye.",
        _idleMsg = "Good bye.",
        _declineMsg = "Then not.",
        _alreadyFocusedMsg = nil, -- nil = don't send (default Jiddo behavior varies)
        _greetWords = {"hi", "hello"},
        _farewellWords = {"bye", "farewell"},
        _keywords = KeywordMatcher:new(),
        _voices = nil,
        _voiceInterval = 10,
        _voiceChance = 10,
        _onSayCallback = nil,
        _onGreetCallback = nil,
        _talkDelay = 1000, -- milliseconds delay before NPC responds (default 1s, matches Jiddo)
        _walkawayMsgMale = nil,
        _walkawayMsgFemale = nil,
        _onFocusAddedCallback = nil,
        _onFocusReleasedCallback = nil,
        _onFarewellCallback = nil,
        _confirmHandlers = {}, -- { [topicName] = { onConfirm = fn, onDecline = fn|nil } }
    }
    setmetatable(obj, self)

    -- Register centralized yes/no keyword handlers (once per builder instance)
    -- Priority 20 to match travel yes/no priority; topic-based dispatch ensures
    -- only the correct handler fires for the current conversation state.
    obj._keywords:addKeyword({"yes"}, function(npc, player, message, builder)
        if not builder:isFocused(npc:getId(), player:getId()) then return false end
        local s = InstanceState.get(npc:getId(), player:getId())
        if not s then return false end
        local handler = builder._confirmHandlers[s.topic]
        if not handler then return false end
        return handler.onConfirm(npc, player, builder, s)
    end, 20)

    obj._keywords:addKeyword({"no"}, function(npc, player, message, builder)
        if not builder:isFocused(npc:getId(), player:getId()) then return false end
        local s = InstanceState.get(npc:getId(), player:getId())
        if not s then return false end
        local handler = builder._confirmHandlers[s.topic]
        if not handler then return false end
        if handler.onDecline then
            return handler.onDecline(npc, player, builder, s)
        end
        -- Default decline: send decline message and reset topic
        s.topic = 0
        builder:say(npc, builder._declineMsg, player)
        InstanceState.updateTalkStart(npc:getId(), player:getId())
        return true
    end, 20)

    return obj
end

-- Register a confirmation handler for a given topic name.
-- onConfirm: function(npc, player, builder, state) -> bool
-- onDecline: function(npc, player, builder, state) -> bool (optional; nil = default decline)
function NpcBuilder:registerConfirmation(topicName, onConfirm, onDecline)
    self._confirmHandlers[topicName] = {
        onConfirm = onConfirm,
        onDecline = onDecline
    }
    return self
end

-- Chainable property setters
function NpcBuilder:health(hp)          self._health = hp; self._maxHealth = hp; return self end
function NpcBuilder:walkInterval(ms)    self._walkInterval = ms; return self end
function NpcBuilder:walkRadius(r)       self._walkRadius = r; return self end
function NpcBuilder:baseSpeed(s)        self._baseSpeed = s; return self end
function NpcBuilder:floorChange(b)      self._floorChange = b; return self end
function NpcBuilder:pushable(b)         self._pushable = b; return self end
function NpcBuilder:speechBubble(b)     self._speechBubble = b; return self end
function NpcBuilder:talkRadius(r)       self._talkRadius = r; return self end
function NpcBuilder:idleTime(s)         self._idleTime = s; return self end

-- Message setters (chainable)
function NpcBuilder:greetMessage(msg)   self._greetMsg = msg; return self end
function NpcBuilder:farewellMessage(msg) self._farewellMsg = msg; return self end
function NpcBuilder:walkawayMessage(msg) self._walkawayMsg = msg; return self end
function NpcBuilder:walkawayMessageMale(msg) self._walkawayMsgMale = msg; return self end
function NpcBuilder:walkawayMessageFemale(msg) self._walkawayMsgFemale = msg; return self end
function NpcBuilder:idleMessage(msg)    self._idleMsg = msg; return self end
function NpcBuilder:declineMessage(msg) self._declineMsg = msg; return self end
function NpcBuilder:alreadyFocusedMessage(msg) self._alreadyFocusedMsg = msg; return self end

-- Custom greet/farewell words (replaces defaults, chainable)
function NpcBuilder:greetWords(words)    self._greetWords = words; return self end
function NpcBuilder:farewellWords(words) self._farewellWords = words; return self end

-- Talk delay in milliseconds (default 1000ms, set to 0 for instant)
function NpcBuilder:talkDelay(ms)        self._talkDelay = ms; return self end

-- Delayed NPC speech helper — queues message with configured delay
function NpcBuilder:say(npc, text, player)
    if self._talkDelay <= 0 then
        npc:say(text, TALKTYPE_PRIVATE_NP, false, player, npc:getPosition())
        return
    end
    local npcId = npc:getId()
    local cid = player:getId()
    addEvent(function(npcId, text, cid)
        local npcObj = Npc(npcId)
        local playerObj = Player(cid)
        if npcObj and playerObj then
            npcObj:say(text, TALKTYPE_PRIVATE_NP, false, playerObj, npcObj:getPosition())
        end
    end, self._talkDelay, npcId, text, cid)
end

-- Multi-message sequence — schedules N events with timed delays
function NpcBuilder:sayMultiple(npc, messages, player, interval)
    interval = interval or 4000
    local npcId = npc:getId()
    local cid = player:getId()

    -- Cancel any existing pending events for this player
    self:cancelPendingEvents(npcId, cid)

    local events = {}
    for i, msg in ipairs(messages) do
        local delay = ((i - 1) * interval) + 700
        local eventId = addEvent(function(npcId, text, cid)
            local npcObj = Npc(npcId)
            local playerObj = Player(cid)
            if npcObj and playerObj then
                npcObj:say(text, TALKTYPE_PRIVATE_NP, false, playerObj, npcObj:getPosition())
            end
        end, delay, npcId, msg, cid)
        events[#events + 1] = eventId
    end

    -- Store event IDs in InstanceState for cancellation on focus release
    InstanceState.setPendingEvents(npcId, cid, events)
end

-- Cancel all pending multi-message events for a player
function NpcBuilder:cancelPendingEvents(npcId, cid)
    local events = InstanceState.getPendingEvents(npcId, cid)
    if events then
        for _, eventId in ipairs(events) do
            stopEvent(eventId)
        end
        InstanceState.clearPendingEvents(npcId, cid)
    end
end

-- Keyword dialogue
function NpcBuilder:addKeyword(keywords, response)
    if type(keywords) == "string" then keywords = {keywords} end
    -- Support table of messages → triggers sayMultiple
    if type(response) == "table" then
        self._keywords:addKeyword(keywords, function(npc, player, message, builder)
            if not builder:isFocused(npc:getId(), player:getId()) then return false end
            builder:sayMultiple(npc, response, player)
            InstanceState.updateTalkStart(npc:getId(), player:getId())
            return true
        end)
    else
        -- Single-message response
        self._keywords:addKeyword(keywords, function(npc, player, message, builder)
            if not builder:isFocused(npc:getId(), player:getId()) then return false end
            local text = response:gsub("|PLAYERNAME|", player:getName())
            builder:say(npc, text, player)
            InstanceState.updateTalkStart(npc:getId(), player:getId())
            return true
        end)
    end
    return self
end

-- Alias keywords — registers the same response under multiple keyword strings
function NpcBuilder:addAliasKeyword(keywordsList, response)
    for _, kw in ipairs(keywordsList) do
        self:addKeyword(kw, response)
    end
    return self
end

-- Topic-aware keywords for multi-step dialogues
function NpcBuilder:addKeywordWithTopic(keywords, response, topicSet, topicRequired)
    if type(keywords) == "string" then keywords = {keywords} end
    self._keywords:addKeyword(keywords, function(npc, player, message, builder)
        if not builder:isFocused(npc:getId(), player:getId()) then return false end
        local s = InstanceState.get(npc:getId(), player:getId())
        if not s then return false end

        -- Check topic prerequisite
        if topicRequired and s.topic ~= topicRequired then return false end

        -- Send response (supports both string and table)
        if type(response) == "table" then
            builder:sayMultiple(npc, response, player)
        else
            local text = response:gsub("|PLAYERNAME|", player:getName())
            builder:say(npc, text, player)
        end

        -- Set new topic
        if topicSet then
            s.topic = topicSet
        end

        InstanceState.updateTalkStart(npc:getId(), player:getId())
        return true
    end)
    return self
end

-- Voice (ambient speech)
function NpcBuilder:addVoice(text, yell)
    if not self._voices then self._voices = {} end
    self._voices[#self._voices + 1] = {
        text = text,
        talktype = yell and TALKTYPE_YELL or TALKTYPE_SAY
    }
    return self
end
function NpcBuilder:voiceInterval(s)  self._voiceInterval = s; return self end
function NpcBuilder:voiceChance(pct)  self._voiceChance = pct; return self end

-- Custom callbacks (chainable)
function NpcBuilder:onSay(callback)   self._onSayCallback = callback; return self end
function NpcBuilder:onGreet(callback) self._onGreetCallback = callback; return self end

-- Exposed state methods for custom callbacks
function NpcBuilder:getTopic(npc, player)
    local s = InstanceState.get(npc:getId(), player:getId())
    return s and s.topic or 0
end

function NpcBuilder:setTopic(npc, player, value)
    local s = InstanceState.get(npc:getId(), player:getId())
    if s then s.topic = value end
end

function NpcBuilder:resetNpc(npc, player)
    local s = InstanceState.get(npc:getId(), player:getId())
    if s then
        s.topic = 0
        s.travelDest = nil
        s.promotionConfig = nil
        s.spellConfig = nil
        s.blessingConfig = nil
        s.shopSelection = nil
    end
end

-- Focus add/release callbacks (chainable)
function NpcBuilder:onFocusAdded(callback)
    self._onFocusAddedCallback = callback; return self
end
function NpcBuilder:onFocusReleased(callback)
    self._onFocusReleasedCallback = callback; return self
end

-- Farewell callback (chainable)
function NpcBuilder:onFarewell(callback)
    self._onFarewellCallback = callback; return self
end

-- Promotion support (full implementation — matches StdModule.promotePlayer)
function NpcBuilder:addPromotion(config)
    -- Register "promot" keyword to initiate confirmation
    self._keywords:addKeyword({"promot"}, function(npc, player, message, builder)
        if not builder:isFocused(npc:getId(), player:getId()) then return false end
        local s = InstanceState.get(npc:getId(), player:getId())
        if not s then return false end
        s.topic = "promotion_confirm"
        s.promotionConfig = config
        builder:say(npc, "I can promote you for " .. config.cost ..
            " gold coins. Do you want me to promote you?", player)
        InstanceState.updateTalkStart(npc:getId(), player:getId())
        return true
    end)

    -- Register confirmation handler via centralized dispatcher
    self:registerConfirmation("promotion_confirm", function(npc, player, builder, s)
        local cfg = s.promotionConfig
        s.topic = 0
        s.promotionConfig = nil

        -- Check already promoted (matches Jiddo: PlayerStorageKeys.promotion == 1)
        if player:getStorageValue(PlayerStorageKeys.promotion) == 1 then
            builder:say(npc, "You are already promoted!", player)
        -- Check level requirement
        elseif player:getLevel() < cfg.level then
            builder:say(npc, "I am sorry, but I can only promote you once you have " ..
                "reached level " .. cfg.level .. ".", player)
        -- Check money
        elseif not player:removeTotalMoney(cfg.cost) then
            builder:say(npc, "You do not have enough money!", player)
        else
            -- Apply promotion
            player:setVocation(player:getVocation():getPromotion())
            player:setStorageValue(PlayerStorageKeys.promotion, 1)
            builder:say(npc, "Congratulations! You are now promoted.", player)
        end
        InstanceState.updateTalkStart(npc:getId(), player:getId())
        return true
    end, function(npc, player, builder, s)
        s.topic = 0
        s.promotionConfig = nil
        builder:say(npc, builder._declineMsg, player)
        InstanceState.updateTalkStart(npc:getId(), player:getId())
        return true
    end)

    return self
end

-- Blessing support (full implementation — matches StdModule.bless)
function NpcBuilder:addBlessing(config)
    local blessId = config.id
    local keyword = config.keyword or "bless"

    self._keywords:addKeyword({keyword:lower()}, function(npc, player, message, builder)
        if not builder:isFocused(npc:getId(), player:getId()) then return false end
        local s = InstanceState.get(npc:getId(), player:getId())
        if not s then return false end

        -- Calculate cost
        local cost
        if config.cost == "level" then
            local level = player:getLevel()
            if level <= 30 then
                cost = 2000
            elseif level < 120 then
                cost = 200 * (level - 20)
            else
                cost = 20000
            end
        else
            cost = config.cost
        end

        s.topic = "blessing_confirm"
        s.blessingConfig = { id = blessId, cost = cost }
        builder:say(npc, "Do you want to receive this blessing for " ..
            cost .. " gold?", player)
        InstanceState.updateTalkStart(npc:getId(), player:getId())
        return true
    end)

    -- Register confirmation handler via centralized dispatcher (once — idempotent)
    if not self._confirmHandlers["blessing_confirm"] then
        self:registerConfirmation("blessing_confirm", function(npc, player, builder, s)
            local cfg = s.blessingConfig
            s.topic = 0
            s.blessingConfig = nil

            if player:hasBlessing(cfg.id) then
                builder:say(npc, "Gods have already blessed you with this blessing!", player)
            elseif not player:removeTotalMoney(cfg.cost) then
                builder:say(npc, "You don't have enough money for blessing.", player)
            else
                player:addBlessing(cfg.id)
                builder:say(npc, "You have been blessed by one of the five gods!", player)
            end
            InstanceState.updateTalkStart(npc:getId(), player:getId())
            return true
        end, function(npc, player, builder, s)
            s.topic = 0
            s.blessingConfig = nil
            builder:say(npc, builder._declineMsg, player)
            InstanceState.updateTalkStart(npc:getId(), player:getId())
            return true
        end)
    end

    return self
end

-- Spell teacher support (full implementation — matches StdModule.learnSpell)
function NpcBuilder:addSpell(config)
    local spellName = config.name
    local spellCost = config.cost

    -- Register keyword matching the spell name
    self._keywords:addKeyword({spellName:lower()}, function(npc, player, message, builder)
        if not builder:isFocused(npc:getId(), player:getId()) then return false end
        local s = InstanceState.get(npc:getId(), player:getId())
        if not s then return false end
        s.topic = "spell_confirm"
        s.spellConfig = config
        builder:say(npc, "Do you want to learn the spell '" .. spellName ..
            "' for " .. spellCost .. " gold?", player)
        InstanceState.updateTalkStart(npc:getId(), player:getId())
        return true
    end)

    -- Register confirmation handler via centralized dispatcher (once — idempotent)
    if not self._confirmHandlers["spell_confirm"] then
        self:registerConfirmation("spell_confirm", function(npc, player, builder, s)
            local cfg = s.spellConfig
            s.topic = 0
            s.spellConfig = nil

            if player:hasLearnedSpell(cfg.name) then
                builder:say(npc, "You already know this spell.", player)
            elseif not player:canLearnSpell(cfg.name) then
                builder:say(npc, "You cannot learn this spell.", player)
            elseif not player:removeTotalMoney(cfg.cost) then
                builder:say(npc, "You do not have enough money, this spell costs " ..
                    cfg.cost .. " gold.", player)
            else
                player:learnSpell(cfg.name)
                builder:say(npc, "You have learned " .. cfg.name .. ".", player)
            end
            InstanceState.updateTalkStart(npc:getId(), player:getId())
            return true
        end, function(npc, player, builder, s)
            s.topic = 0
            s.spellConfig = nil
            builder:say(npc, builder._declineMsg, player)
            InstanceState.updateTalkStart(npc:getId(), player:getId())
            return true
        end)
    end

    return self
end

---------------------------------------------------------------------------
-- Focus Management (delegates to InstanceState with per-spawn creature ID)
---------------------------------------------------------------------------

function NpcBuilder:isFocused(npcId, cid)
    return InstanceState.isFocused(npcId, cid)
end

function NpcBuilder:addFocus(npc, cid)
    local npcId = npc:getId()
    if InstanceState.isFocused(npcId, cid) then return end

    -- Invoke onFocusAdded callback; cancel if returns false
    if self._onFocusAddedCallback then
        local player = Player(cid)
        if player and self._onFocusAddedCallback(npc, player, self) == false then
            return
        end
    end

    InstanceState.setFocus(npcId, cid)
    InstanceState.create(npcId, cid)
    local creature = Creature(cid)
    if creature then
        doNpcSetCreatureFocus(cid)
    end
end

function NpcBuilder:releaseFocus(npc, cid)
    local npcId = npc:getId()
    if not InstanceState.isFocused(npcId, cid) then return end

    -- Invoke onFocusReleased callback; cancel if returns false
    if self._onFocusReleasedCallback then
        local player = Player(cid)
        if player and self._onFocusReleasedCallback(npc, player, self) == false then
            return
        end
    end

    -- Cancel pending multi-message events before releasing
    self:cancelPendingEvents(npcId, cid)

    InstanceState.clearFocus(npcId, cid)
    InstanceState.remove(npcId, cid)
    -- Use the global closeShopWindow (from Jiddo compat) which handles nil safely
    if closeShopWindow then
        closeShopWindow(cid)
    end
    -- Update focus to next player or nil
    for focusCid, _ in pairs(InstanceState.getFocuses(npcId)) do
        doNpcSetCreatureFocus(focusCid)
        return
    end
    doNpcSetCreatureFocus(0)
end

function NpcBuilder:isInRange(npc, cid)
    local player = Player(cid)
    if not player then return false end
    local npcPos = npc:getPosition()
    local playerPos = player:getPosition()
    if npcPos.z ~= playerPos.z then return false end
    local dist = math.max(math.abs(npcPos.x - playerPos.x),
                          math.abs(npcPos.y - playerPos.y))
    return dist <= self._talkRadius
end

---------------------------------------------------------------------------
-- Event Handlers
---------------------------------------------------------------------------

function NpcBuilder:handleAppear(npc, creature)
    -- Register this spawn's creature ID with InstanceState when the NPC itself appears
    if creature and creature:getId() == npc:getId() then
        InstanceState.register(npc:getId())
        self:onNpcAppear(npc)
    end
end

function NpcBuilder:onNpcAppear(npc)
    -- Subclasses override (e.g., ShopBuilder sets SPEECHBUBBLE_TRADE)
end

function NpcBuilder:handleDisappear(npc, creature)
    if not creature then return end
    local cid = creature:getId()
    if cid == npc:getId() then return end
    if self:isFocused(npc:getId(), cid) then
        self:releaseFocus(npc, cid)
    end
end

function NpcBuilder:handleSay(npc, creature, msgtype, message)
    if not creature then return end
    local cid = creature:getId()
    local player = Player(cid)
    if not player then return end
    if not self:isInRange(npc, cid) then return end

    local npcId = npc:getId()
    local lower = message:lower()

    -- Greet handling (substring match using configurable greet words)
    if not self:isFocused(npcId, cid) then
        for _, word in ipairs(self._greetWords) do
            if lower:find(word, 1, true) then
                self:greet(npc, player)
                return
            end
        end
        return
    end

    -- Already focused — player said "hi" again while talking
    for _, word in ipairs(self._greetWords) do
        if lower:find(word, 1, true) then
            if self._alreadyFocusedMsg then
                local text = self._alreadyFocusedMsg:gsub("|PLAYERNAME|", player:getName())
                self:say(npc, text, player)
            end
            InstanceState.updateTalkStart(npcId, cid)
            return
        end
    end

    -- Farewell handling (exact match using configurable farewell words)
    for _, word in ipairs(self._farewellWords) do
        if lower == word then
            self:farewell(npc, player)
            return
        end
    end

    -- Try keyword matcher first
    if self._keywords:match(message, npc, player, self) then
        InstanceState.updateTalkStart(npcId, cid)
        return
    end

    -- Try custom onSay callback
    if self._onSayCallback then
        if self._onSayCallback(npc, player, message, self) then
            InstanceState.updateTalkStart(npcId, cid)
            return
        end
    end
end

function NpcBuilder:greet(npc, player)
    local cid = player:getId()
    -- Custom greet callback
    if self._onGreetCallback then
        local result = self._onGreetCallback(npc, player, self)
        if result == false then return end -- cancelled
    end
    self:addFocus(npc, cid)
    local text = self._greetMsg:gsub("|PLAYERNAME|", player:getName())
    self:say(npc, text, player)
end

function NpcBuilder:farewell(npc, player)
    local cid = player:getId()

    -- Invoke onFarewell callback; cancel if returns false
    if self._onFarewellCallback then
        if self._onFarewellCallback(npc, player, self) == false then
            return
        end
    end

    local text = self._farewellMsg:gsub("|PLAYERNAME|", player:getName())
    self:say(npc, text, player)
    self:releaseFocus(npc, cid)
end

function NpcBuilder:handleThink(npc, interval)
    local npcId = npc:getId()

    -- Voice module
    if self._voices and #self._voices > 0 then
        if InstanceState.getLastVoice(npcId) < os.time() then
            InstanceState.setLastVoice(npcId, os.time() + self._voiceInterval)
            if math.random(100) <= self._voiceChance then
                local voice = self._voices[math.random(#self._voices)]
                npc:say(voice.text, voice.talktype)
            end
        end
    end

    -- Idle/walkaway checks (per-spawn focus list from InstanceState)
    for cid, _ in pairs(InstanceState.getFocuses(npcId)) do
        local player = Player(cid)
        if not player then
            -- Player logged out or disconnected — clean up
            InstanceState.clearFocus(npcId, cid)
            InstanceState.remove(npcId, cid)
            -- Reset NPC visual focus
            local nextFocus = next(InstanceState.getFocuses(npcId))
            doNpcSetCreatureFocus(nextFocus or 0)
        elseif not self:isInRange(npc, cid) then
            -- Walkaway
            local player = Player(cid)
            if player then
                -- Gender-specific walkaway (matches Jiddo npchandler.lua:330-355)
                if self._walkawayMsgMale and self._walkawayMsgFemale
                   and self._walkawayMsgMale ~= self._walkawayMsgFemale then
                    local sex = player:getSex()
                    if sex == PLAYERSEX_FEMALE then
                        self:say(npc, self._walkawayMsgFemale:gsub("|PLAYERNAME|", player:getName()), player)
                    else
                        self:say(npc, self._walkawayMsgMale:gsub("|PLAYERNAME|", player:getName()), player)
                    end
                else
                    local text = self._walkawayMsg:gsub("|PLAYERNAME|", player:getName())
                    self:say(npc, text, player)
                end
            end
            self:releaseFocus(npc, cid)
        elseif InstanceState.getTalkStart(npcId, cid) and
               (os.time() - InstanceState.getTalkStart(npcId, cid)) > self._idleTime then
            -- Idle timeout
            local player = Player(cid)
            if player then
                self:say(npc, self._idleMsg, player)
            end
            self:releaseFocus(npc, cid)
        else
            doNpcSetCreatureFocus(cid)
        end
    end
end

function NpcBuilder:handleCloseChannel(npc, creature)
    if not creature then return end
    local cid = creature:getId()
    if self:isFocused(npc:getId(), cid) then
        self:releaseFocus(npc, cid)
    end
end

---------------------------------------------------------------------------
-- register() — Creates NpcType, configures properties, wires EventDispatcher
---------------------------------------------------------------------------

function NpcBuilder:register()
    local npcType = Game.createNpcType(self._name)

    -- Configure NpcType properties
    npcType:name(self._name)
    npcType:health(self._health)
    npcType:maxHealth(self._maxHealth)
    npcType:walkInterval(self._walkInterval)
    npcType:walkRadius(self._walkRadius)
    npcType:baseSpeed(self._baseSpeed)
    npcType:floorChange(self._floorChange)
    npcType:isPushable(self._pushable)
    npcType:outfit(self._outfit)
    npcType:speechBubble(self._speechBubble)

    -- Register with EventDispatcher
    EventDispatcher.register(self._name, self)
    EventDispatcher.setupCallbacks(npcType, self._name)

    -- Finalize
    npcType:register()
    return self
end
