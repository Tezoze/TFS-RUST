-- TravelBuilder: Specialized NPC builder for boat captains and carpet riders
-- Extends NpcBuilder with destination management, confirmation flow,
-- premium/PZ checks, and teleportation with magic effects.

TravelBuilder = setmetatable({}, { __index = NpcBuilder })
TravelBuilder.__index = TravelBuilder

function TravelBuilder:new(name, outfit)
    local obj = NpcBuilder.new(self, name, outfit)
    obj._destinations = {}  -- { {name, position, cost, premium}, ... }
    obj._postmanDiscount = false
    setmetatable(obj, self)
    return obj
end

function TravelBuilder:addDestination(name, position, cost, premium, level)
    self._destinations[#self._destinations + 1] = {
        name = name,
        position = position,
        cost = cost,
        premium = premium or false,
        level = level or nil
    }
    return self
end

function TravelBuilder:postmanDiscount(enabled)
    self._postmanDiscount = enabled; return self
end

-- Helper: calculate postman discount for a player
function TravelBuilder:calculatePostmanDiscount(player)
    if not self._postmanDiscount then return 0 end
    local discount = 0
    for i = 1, 10 do
        local key = "Mission" .. string.format("%02d", i)
        if player:getStorageValue(Storage.postman[key]) == 1 then
            discount = discount + 10
        end
    end
    return discount
end

function TravelBuilder:register()
    local builder = self

    -- "travel"/"destination"/"where" keyword lists destinations
    local listFn = function(npc, player, message, b)
        if not b:isFocused(npc:getId(), player:getId()) then return false end
        local msg = "I can bring you to "
        for i, dest in ipairs(b._destinations) do
            msg = msg .. dest.name
            if i == #b._destinations - 1 then
                msg = msg .. " and "
            elseif i < #b._destinations then
                msg = msg .. ", "
            else
                msg = msg .. "."
            end
        end
        b:say(npc, msg, player)
        InstanceState.updateTalkStart(npc:getId(), player:getId())
        return true
    end
    self._keywords:addKeyword({"travel"}, listFn, 5)
    self._keywords:addKeyword({"destination"}, listFn, 5)
    self._keywords:addKeyword({"where"}, listFn, 5)

    -- Each destination name triggers a confirmation prompt
    for _, dest in ipairs(self._destinations) do
        self._keywords:addKeyword({dest.name:lower()}, function(npc, player, message, b)
            if not b:isFocused(npc:getId(), player:getId()) then return false end
            local s = InstanceState.get(npc:getId(), player:getId())
                or InstanceState.create(npc:getId(), player:getId())
            local discount = b:calculatePostmanDiscount(player)
            local finalCost = math.max(0, dest.cost - discount)
            s.topic = "travel_confirm"
            s.travelDest = { name = dest.name, position = dest.position,
                             cost = finalCost, premium = dest.premium, level = dest.level }
            b:say(npc, "Do you want to travel to " .. dest.name ..
                " for " .. finalCost .. " gold coins?", player)
            InstanceState.updateTalkStart(npc:getId(), player:getId())
            return true
        end)
    end

    -- Register confirmation handler via centralized dispatcher
    self:registerConfirmation("travel_confirm", function(npc, player, builder, s)
        local dest = s.travelDest
        s.topic = 0
        s.travelDest = nil

        if dest.premium and not player:isPremium() then
            builder:say(npc, "I can only allow premium players to travel there.", player)
            return true
        end
        if player:isPzLocked() then
            builder:say(npc, "First get rid of those blood stains! You are not going to ruin my vehicle!", player)
            return true
        end
        if dest.level and player:getLevel() < dest.level then
            builder:say(npc, "You must reach level " .. dest.level ..
                " before I can let you go there.", player)
            return true
        end
        if not player:removeTotalMoney(dest.cost) then
            builder:say(npc, "You don't have enough money.", player)
            return true
        end
        builder:say(npc, "Set the sails!", player)
        builder:releaseFocus(npc, player:getId())
        local fromPos = player:getPosition()
        player:teleportTo(Position(dest.position))
        fromPos:sendMagicEffect(CONST_ME_TELEPORT)
        Position(dest.position):sendMagicEffect(CONST_ME_TELEPORT)
        return true
    end, function(npc, player, builder, s)
        s.topic = 0
        s.travelDest = nil
        builder:say(npc, "Then not.", player)
        InstanceState.updateTalkStart(npc:getId(), player:getId())
        return true
    end)

    NpcBuilder.register(self)
    return self
end
