-- Tom - Converted from XML to Lua NpcType
-- Original XML: data/npc/Tom.xml
-- Original Script: data/npc/scripts/Tom.lua

local npcName = "Tom"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a tom")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 144, lookHead = 113, lookBody = 115, lookLegs = 58, lookFeet = 115, lookAddons = 1})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local voices = {
	{ text = 'Buying fresh corpses of rats, rabbits and wolves.' },
	{ text = 'Oh yeah, I\'m also interested in wolf paws and bear paws.' },
	{ text = 'Also buying minotaur leather.' }
}
npcHandler:addModule(VoiceModule:new(voices))

-- Greeting and Farewell
keywordHandler:addGreetKeyword({'hi'}, {npcHandler = npcHandler, text = 'Hey there, |PLAYERNAME|. I\'m Tom the tanner. If you have fresh {corpses}, leather, paws or other animal body parts, {trade} with me.'})
keywordHandler:addAliasKeyword({'hello'})
keywordHandler:addFarewellKeyword({'bye'}, {npcHandler = npcHandler, text = 'Good hunting, child.'}, function(player) return player:getSex() == PLAYERSEX_FEMALE end)
keywordHandler:addAliasKeyword({'farewell'})
keywordHandler:addFarewellKeyword({'bye'}, {npcHandler = npcHandler, text = 'Good hunting, son.'})
keywordHandler:addAliasKeyword({'farewell'})

-- Basic keywords
keywordHandler:addKeyword({'hint'}, StdModule.rookgaardHints, {npcHandler = npcHandler})
keywordHandler:addKeyword({'name'}, StdModule.say, {npcHandler = npcHandler, text = 'My name is Tom the tanner.'})
keywordHandler:addKeyword({'name'}, StdModule.say, {npcHandler = npcHandler, text = 'I\'m the local {tanner}. I buy fresh animal {corpses}, tan them, and convert them into fine leather clothes which I then sell to {merchants}.'})
keywordHandler:addKeyword({'merchant'}, StdModule.say, {npcHandler = npcHandler, text = '{Dixi} and {Lee\'Delle} sell my leather clothes in their shops.'})
keywordHandler:addKeyword({'tanner'}, StdModule.say, {npcHandler = npcHandler, text = 'That\'s my job. It can be dirty at times but it provides enough income for my living.'})
keywordHandler:addKeyword({'information'}, StdModule.say, {npcHandler = npcHandler, text = 'Do I look like a tourist information centre? Go ask someone else.'})
keywordHandler:addKeyword({'help'}, StdModule.say, {npcHandler = npcHandler, text = 'Help? I will give you a few gold coins if you have some fresh dead {animals} for me. Note the word {fresh}.'})
keywordHandler:addKeyword({'fresh'}, StdModule.say, {npcHandler = npcHandler, text = 'Fresh means: shortly after their death.'})
keywordHandler:addKeyword({'how', 'are', 'you'}, StdModule.say, {npcHandler = npcHandler, text = 'Much to do these days.'})
keywordHandler:addKeyword({'monster'}, StdModule.say, {npcHandler = npcHandler, text = 'Good monsters to start with are rats. They live in the {sewers} under the village of {Rookgaard}.'})
keywordHandler:addKeyword({'dungeon'}, StdModule.say, {npcHandler = npcHandler, text = 'Dungeons can be dangerous without proper {equipment}.'})
keywordHandler:addKeyword({'equipment'}, StdModule.say, {npcHandler = npcHandler, text = 'You need at least a {backpack}, a {rope}, a {shovel}, a {weapon}, an {armor} and a {shield}.'})
keywordHandler:addKeyword({'time'}, StdModule.say, {npcHandler = npcHandler, text = 'Sorry, I haven\'t been outside for a while, so I don\'t know.'})

keywordHandler:addKeyword({'troll'}, StdModule.say, {npcHandler = npcHandler, text = 'Troll leather stinks. Can\'t use it.'})
keywordHandler:addKeyword({'orc'}, StdModule.say, {npcHandler = npcHandler, text = 'I don\'t buy orcs. Their skin is too scratchy.'})
keywordHandler:addKeyword({'human'}, StdModule.say, {npcHandler = npcHandler, text = 'Are you crazy?!', ungreet = true})

keywordHandler:addKeyword({'backpack'}, StdModule.say, {npcHandler = npcHandler, text = 'Nope, sorry, don\'t sell that. Go see {Al Dee} or {Lee\'Delle}.'})
keywordHandler:addAliasKeyword({'rope'})

keywordHandler:addKeyword({'armor'}, StdModule.say, {npcHandler = npcHandler, text = 'Nope, sorry, don\'t sell that. Ask {Dixi} or {Lee\'Delle}.'})
keywordHandler:addAliasKeyword({'shield'})

keywordHandler:addKeyword({'weapon'}, StdModule.say, {npcHandler = npcHandler, text = 'Nope, sorry, don\'t sell that. Ask {Obi} or {Lee\'Delle}.'})

keywordHandler:addKeyword({'corpse'}, StdModule.say, {npcHandler = npcHandler, text = 'I\'m buying fresh {corpses} of rats, rabbits and wolves. I don\'t buy half-decayed ones. If you have any for sale, {trade} with me.'})
keywordHandler:addAliasKeyword({'wares'})
keywordHandler:addAliasKeyword({'animal'})
keywordHandler:addAliasKeyword({'sell'})
keywordHandler:addAliasKeyword({'buy'})
keywordHandler:addAliasKeyword({'offer'})

-- Names
keywordHandler:addKeyword({'al', 'dee'}, StdModule.say, {npcHandler = npcHandler, text = 'He\'s an apple polisher.'})
keywordHandler:addKeyword({'amber'}, StdModule.say, {npcHandler = npcHandler, text = 'Now that\'s an interesting woman.'})
keywordHandler:addKeyword({'billy'}, StdModule.say, {npcHandler = npcHandler, text = 'He\'s a better cook than his cousin {Willie}, actually.'})
keywordHandler:addKeyword({'willie'}, StdModule.say, {npcHandler = npcHandler, text = 'I kinda like him. At least he says what he thinks.'})
keywordHandler:addKeyword({'tom'}, StdModule.say, {npcHandler = npcHandler, text = 'Yep.'})
keywordHandler:addKeyword({'seymour'}, StdModule.say, {npcHandler = npcHandler, text = 'He sticks his nose too much in books.'})
keywordHandler:addKeyword({'zirella'}, StdModule.say, {npcHandler = npcHandler, text = 'My mother?? Did you meet my mother??'})
keywordHandler:addKeyword({'santiago'}, StdModule.say, {npcHandler = npcHandler, text = 'I don\'t have a problem with him.'})
keywordHandler:addKeyword({'paulie'}, StdModule.say, {npcHandler = npcHandler, text = 'Typical pencil pusher.'})
keywordHandler:addKeyword({'oracle'}, StdModule.say, {npcHandler = npcHandler, text = 'It\'s in the academy, just above Seymour. Go there once you are level 8 to leave this place.'})
keywordHandler:addKeyword({'obi'}, StdModule.say, {npcHandler = npcHandler, text = 'He is such a hypocrite.'})
keywordHandler:addKeyword({'norma'}, StdModule.say, {npcHandler = npcHandler, text = 'I like her beer.'})
keywordHandler:addKeyword({'dixi'}, StdModule.say, {npcHandler = npcHandler, text = 'She buys my fine leather clothes.'})
keywordHandler:addKeyword({'loui'}, StdModule.say, {npcHandler = npcHandler, text = 'I wonder what spectacular monsters he has found.'})
keywordHandler:addKeyword({'lee\'delle'}, StdModule.say, {npcHandler = npcHandler, text = 'Her nose is a little high in the air, I think. She never shakes my hand.'})
keywordHandler:addKeyword({'hyacinth'}, StdModule.say, {npcHandler = npcHandler, text = 'I wonder if he\'s angry because his potion monopoly fell.'})
keywordHandler:addKeyword({'cipfried'}, StdModule.say, {npcHandler = npcHandler, text = 'I\'m not what you\'d call a \'believer\'.'})
keywordHandler:addKeyword({'dallheim'}, StdModule.say, {npcHandler = npcHandler, text = 'He\'s okay.'})
keywordHandler:addAliasKeyword({'zerbrus'})

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	if msgcontains(msg, "cough syrup") then
		npcHandler:say("I had some cough syrup a while ago. It was stolen in an ape raid. I fear if you want more cough syrup you will have to buy it in the druids guild in carlin.", cid)
	elseif msgcontains(msg, "addon") then
		if player:getStorageValue(Storage.OutfitQuest.DruidBodyAddon) < 1 then
			npcHandler:say("Would you like to wear bear paws like I do? No problem, just bring me 50 bear paws and 50 wolf paws and I'll fit them on.", cid)
			player:setStorageValue(Storage.OutfitQuest.DruidBodyAddon, 1)
			npcHandler.topic[cid] = 0
		end
	elseif msgcontains(msg, "paws") or msgcontains(msg, "bear paws") then
		if player:getStorageValue(Storage.OutfitQuest.DruidBodyAddon) == 1 then
			npcHandler:say("Have you brought 50 bear paws and 50 wolf paws?", cid)
			npcHandler.topic[cid] = 1
		end
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 1 then
			if player:getItemCount(5896) >= 50 and player:getItemCount(5897) >= 50 then
				npcHandler:say("Excellent! Like promised, here are your bear paws. ", cid)
				player:removeItem(5896, 50)
				player:removeItem(5897, 50)
				player:setStorageValue(Storage.OutfitQuest.DruidBodyAddon, 2)
				player:addOutfitAddon(148, 1)
				player:addOutfitAddon(144, 1)
				npcHandler.topic[cid] = 0
			else
				npcHandler:say("You don't have all items.", cid)
				npcHandler.topic[cid] = 0
			end
		end
	end
	return true
end

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
npcHandler:setMessage(MESSAGE_WALKAWAY, 'D\'oh?')
npcHandler:setMessage(MESSAGE_SENDTRADE, 'Sure, check what I buy.')


-- Shop items (from XML parameters)
local shopItems = {
    {id = 5896, buy = 0, sell = 10, subType = 0, name = "bear paw"},
    {id = 2992, buy = 0, sell = 2, subType = 0, name = "dead rabbit"},
    {id = 2813, buy = 0, sell = 2, subType = 0, name = "dead rat"},
    {id = 2826, buy = 0, sell = 5, subType = 0, name = "dead wolf"},
    {id = 5878, buy = 0, sell = 12, subType = 0, name = "minotaur leather"},
    {id = 5897, buy = 0, sell = 7, subType = 0, name = "wolf paw"},
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
