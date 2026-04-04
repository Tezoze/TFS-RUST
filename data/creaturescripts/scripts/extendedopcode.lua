local OPCODE_LANGUAGE = 1
local SHOP_EXTENDED_OPCODE = 201
local AUTOLOOT_OPCODE = 202

-- Storage key for shop points
local STORAGE_SHOP_POINTS = 10001

function onExtendedOpcode(player, opcode, buffer)
	print("[ExtendedOpcode DEBUG] Received opcode " .. opcode .. " from " .. player:getName() .. " buffer_len=" .. #buffer)
	if opcode == OPCODE_LANGUAGE then
		-- otclient language
		if buffer == 'en' or buffer == 'pt' then
			-- example, setting player language, because otclient is multi-language...
			-- player:setStorageValue(SOME_STORAGE_ID, SOME_VALUE)
		end
	elseif opcode == SHOP_EXTENDED_OPCODE then
		-- Handle shop purchases
		local success, data = pcall(function() return json.decode(buffer) end)
		if not success or not data then
			return true
		end

		if data.action == "buy" then
			handleShopPurchase(player, data)
		elseif data.action == "init" then
			-- Send shop initialization data if needed
			sendShopInit(player)
		end
	elseif opcode == AUTOLOOT_OPCODE then
		-- Forward to the autoloot handler defined in data/scripts/custom/autoloot.lua
		if handleAutoLootOpcode then
			handleAutoLootOpcode(player, opcode, buffer)
		else
			print("[AutoLoot] WARNING: handleAutoLootOpcode not found - autoloot.lua may not be loaded")
		end
	else
		-- other opcodes can be ignored, and the server will just work fine...
	end
	return true
end

function handleShopPurchase(player, data)
	local itemId = data.itemId
	local cost = data.cost
	local itemName = data.title or "Unknown Item"

	-- Validate purchase data
	if not itemId or not cost or cost <= 0 then
		player:sendTextMessage(MESSAGE_STATUS_WARNING, "Invalid purchase data.")
		return
	end

	-- Check if player has enough points
	local playerPoints = player:getStorageValue(STORAGE_SHOP_POINTS)
	if playerPoints == -1 then
		playerPoints = 0 -- Player hasn't deposited any coins yet
	end

	if playerPoints < cost then
		player:sendTextMessage(MESSAGE_STATUS_WARNING, "You don't have enough points. You need " .. cost .. " but only have " .. playerPoints .. ".")
		return
	end

	-- Check if item is a valid store item
	local itemType = ItemType(itemId)
	if not itemType or not itemType:isStoreItem() then
		player:sendTextMessage(MESSAGE_STATUS_WARNING, "This item cannot be purchased.")
		return
	end

	-- Process the purchase - deduct points from storage
	player:setStorageValue(STORAGE_SHOP_POINTS, playerPoints - cost)

	-- Try to add item to Store Inbox
	local storeInbox = player:getSlotItem(CONST_SLOT_STORE_INBOX)
	if storeInbox then
		local added = storeInbox:addItem(itemId, 1)
		if added then
			player:sendTextMessage(MESSAGE_INFO_DESCR, "You have successfully purchased " .. itemName .. " for " .. cost .. " points. The item has been added to your Store Inbox.")
			sendShopStatus(player) -- Update client points balance
		else
			player:sendTextMessage(MESSAGE_STATUS_WARNING, "Failed to add item to Store Inbox. Please try again.")
			-- Refund the points
			player:setStorageValue(STORAGE_SHOP_POINTS, playerPoints)
		end
	else
		player:sendTextMessage(MESSAGE_STATUS_WARNING, "Store Inbox not found. Please contact an administrator.")
		-- Refund the points
		player:setStorageValue(STORAGE_SHOP_POINTS, playerPoints)
	end
end

function sendShopInit(player)
	-- Send basic shop initialization data
	local points = player:getStorageValue(STORAGE_SHOP_POINTS)
	if points == -1 then
		points = 0 -- Player hasn't deposited any coins yet
	end

	local shopInit = {
		action = "init",
		status = {
			points = points,
			buyUrl = ""
		}
	}

	local encodedData = json.encode(shopInit)
	player:sendExtendedOpcode(SHOP_EXTENDED_OPCODE, encodedData)
end

function sendShopStatus(player)
	-- Send updated shop status (points balance)
	local points = player:getStorageValue(STORAGE_SHOP_POINTS)
	if points == -1 then
		points = 0 -- Player hasn't deposited any coins yet
	end

	local shopStatus = {
		action = "status",
		status = {
			points = points
		}
	}

	local encodedData = json.encode(shopStatus)
	player:sendExtendedOpcode(SHOP_EXTENDED_OPCODE, encodedData)
end

-- Container Memory functions are now in data/lib/container_memory.lua
