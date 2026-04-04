-- Talkaction to manually refresh access token descriptions
-- Usage: /refreshtokens

local accessTokenDescriptions = {
	[40001] = "Efreet Djinn Access Token\n\nGrants access to trade with Efreet faction djinns! Completes all Efreet missions.",
	[40002] = "Marid Djinn Access Token\n\nGrants access to trade with Marid faction djinns! Completes all Marid missions.",
	[40003] = "Travelling Trader Access Token\n\nGrants access to trade with Rashid! Completes all Travelling Trader missions.",
	[40004] = "In Service of Yalahar Access Token\n\nCompletes all In Service of Yalahar missions except the final battle!",
	[40005] = "The New Frontier Access Token\n\nCompletes all The New Frontier missions except Mortal Combat!",
	[40006] = "Farmine Access Token\n\nCompletes Children of the Revolution and Wrath of the Emperor up to Mission 11!"
}

local function checkAndSetDescription(item)
	if not item then
		return false
	end
	
	local actionId = item:getActionId()
	if actionId and accessTokenDescriptions[actionId] then
		item:setAttribute(ITEM_ATTRIBUTE_DESCRIPTION, accessTokenDescriptions[actionId])
		return true
	end
	return false
end

local function checkContainer(container)
	local count = 0
	if not container then
		return count
	end
	
	for i = 0, container:getSize() - 1 do
		local item = container:getItem(i)
		if item then
			if checkAndSetDescription(item) then
				count = count + 1
			end
			if item:isContainer() then
				count = count + checkContainer(item)
			end
		end
	end
	return count
end

local refreshTokens = TalkAction("/refreshtokens")

function refreshTokens.onSay(player, words, param)
	local count = 0
	
	-- Check all equipped items and containers
	for slot = CONST_SLOT_FIRST, CONST_SLOT_LAST do
		local item = player:getSlotItem(slot)
		if item then
			if checkAndSetDescription(item) then
				count = count + 1
			end
			if item:isContainer() then
				count = count + checkContainer(item)
			end
		end
	end
	
	if count > 0 then
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Updated descriptions for " .. count .. " access token(s)!")
	else
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "No access tokens found in your inventory.")
	end
	
	return false
end

refreshTokens:separator(" ")
refreshTokens:register()
