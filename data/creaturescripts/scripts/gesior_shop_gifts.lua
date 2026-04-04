-- Gesior Shop System - Gift Processing Script
-- This script processes pending gifts from the myaac gesior shop system
-- Gifts are stored in z_ots_comunication table and processed on player login

function processGesiorShopGifts(player)
	local playerName = player:getName()
	local playerId = player:getGuid()

	-- Query for pending gifts in z_ots_comunication table
	-- Join with z_shop_history to get the offer_id, then join with z_shop_offer to get action_id
	local resultId = db.storeQuery([[
		SELECT 
			c.`id`, 
			c.`param1`, 
			c.`param2`, 
			c.`param3`, 
			c.`param4`, 
			c.`param5`, 
			c.`param6`,
			o.`action_id`
		FROM `z_ots_comunication` c
		LEFT JOIN `z_shop_history` h ON h.`comunication_id` = c.`id`
		LEFT JOIN `z_shop_offer` o ON o.`id` = h.`offer_id`
		WHERE c.`name` = ]] .. db.escapeString(playerName) .. [[ 
		AND c.`type` = 'login' 
		AND c.`action` = 'give_item' 
		AND c.`delete_it` = 1
	]])

	if resultId ~= false then
		repeat
			local giftId = result.getNumber(resultId, "id")
			local param1 = result.getNumber(resultId, "param1")
			local param2 = result.getNumber(resultId, "param2")
			local param3 = result.getNumber(resultId, "param3")
			local param4 = result.getNumber(resultId, "param4")
			local giftType = result.getString(resultId, "param5")
			local offerName = result.getString(resultId, "param6")
			local actionId = result.getNumber(resultId, "action_id")

			if giftType == "item" then
				-- param1 = item_id, param2 = count, actionId from z_shop_offer
				if param1 > 0 and param2 > 0 then
					processItemGift(player, playerId, param1, param2, actionId)
				end

			elseif giftType == "container" then
				-- param1 = item_id, param2 = item_count, param3 = container_id, param4 = container_count
				if param3 > 0 and param4 > 0 then
					processContainerGift(player, playerId, param1, param2, param3, param4)
				end

			elseif giftType == "addon" then
				-- param1 = look_female, param2 = look_male, param3 = addons_female, param4 = addons_male
				processAddonGift(player, param1, param2, param3, param4)

			elseif giftType == "mount" then
				-- param1 = mount_id
				if param1 > 0 then
					processMountGift(player, param1)
				end
			end

			-- Mark gift as processed (delete from communication table)
			db.query("DELETE FROM `z_ots_comunication` WHERE `id` = " .. giftId)

			-- Update shop history to mark transaction as realized
			db.query("UPDATE `z_shop_history` SET `trans_state` = 'realized', `trans_real` = " .. os.time() .. " WHERE `comunication_id` = " .. giftId)

		until not result.next(resultId)
		result.free(resultId)
	end
end

-- Access token descriptions mapping
local accessTokenDescriptions = {
	[40001] = "Efreet Djinn Access Token\n\nGrants access to trade with Efreet faction djinns! Completes all Efreet missions.",
	[40002] = "Marid Djinn Access Token\n\nGrants access to trade with Marid faction djinns! Completes all Marid missions.",
	[40003] = "Travelling Trader Access Token\n\nGrants access to trade with Rashid! Completes all Travelling Trader missions.",
	[40004] = "In Service of Yalahar Access Token\n\nCompletes all In Service of Yalahar missions except the final battle!",
	[40005] = "The New Frontier Access Token\n\nCompletes all The New Frontier missions except Mortal Combat!",
	[40006] = "Farmine Access Token\n\nCompletes Children of the Revolution and Wrath of the Emperor up to Mission 11!",
	[40007] = "The Ape City Access Token\n\nCompletes The Ape City quest and grants the Shaman outfit!",
	[40008] = "Deeper Banuta Shortcut Token\n\nGrants access to the Deeper Banuta shortcut!",
	[40009] = "Pits of Inferno Shortcut Token\n\nGrants access to the Pits of Inferno shortcut!"
}

function processItemGift(player, playerId, itemId, count, actionId)
	-- Add to Venore depot chest (town ID 1) - first chest in universal depot
	local townId = 1 -- Venore
	local depotChest = player:getDepotChest(townId, true)
	
	if depotChest then
		-- Create item with FLAG_NOLIMIT to bypass capacity checks
		local item = Game.createItem(itemId, count)
		if item then
			-- Set action ID before adding to depot
			if actionId and actionId > 0 then
				item:setActionId(actionId)
				
				-- Set description for access tokens
				if accessTokenDescriptions[actionId] then
					item:setAttribute(ITEM_ATTRIBUTE_DESCRIPTION, accessTokenDescriptions[actionId])
				end
			end
			
			local ret = depotChest:addItemEx(item, INDEX_WHEREEVER, FLAG_NOLIMIT)
			
			if ret == RETURNVALUE_NOERROR then
				player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have received " .. item:getName() .. " in your depot!")
			else
				item:remove() -- Clean up the item if it couldn't be added
			end
		end
	end
end

function processContainerGift(player, playerId, itemId, itemCount, containerId, containerCount)
	-- Add container to Venore depot chest (town ID 1)
	local townId = 1 -- Venore
	local depotChest = player:getDepotChest(townId, true)
	if depotChest then
		local addedContainer = depotChest:addItem(containerId, containerCount)
		if addedContainer then
			-- If container has items inside, add them as children
			if itemId > 0 and itemCount > 0 and addedContainer:isContainer() then
				addedContainer:addItem(itemId, itemCount)
			end
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have received " .. addedContainer:getName() .. " in your depot!")
		end
	end
end

function processAddonGift(player, lookFemale, lookMale, addonsFemale, addonsMale)
	-- Addon gifts - unlock outfits and addons directly for the player
	-- Addon values: 0 = base outfit only, 1 = first addon, 2 = second addon, 3 = both addons
	local playerSex = player:getSex() -- 0 = female, 1 = male
	
	if playerSex == 0 and lookFemale > 0 then -- Female player
		if addonsFemale == 0 then
			-- Just give the base outfit (no addons)
			player:addOutfit(lookFemale)
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have received a new outfit!")
		elseif addonsFemale > 0 then
			-- Give the outfit with addons
			if player:addOutfitAddon(lookFemale, addonsFemale) then
				player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have received new outfit addons!")
			else
				-- Fallback: just give the base outfit if addon fails
				player:addOutfit(lookFemale)
				player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have received a new outfit!")
			end
		end
	elseif playerSex == 1 and lookMale > 0 then -- Male player
		if addonsMale == 0 then
			-- Just give the base outfit (no addons)
			player:addOutfit(lookMale)
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have received a new outfit!")
		elseif addonsMale > 0 then
			-- Give the outfit with addons
			if player:addOutfitAddon(lookMale, addonsMale) then
				player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have received new outfit addons!")
			else
				-- Fallback: just give the base outfit if addon fails
				player:addOutfit(lookMale)
				player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have received a new outfit!")
			end
		end
	end
end

function processMountGift(player, mountId)
	-- Mount gifts - unlock mount access directly for the player
	if mountId > 0 then
		if player:addMount(mountId) then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have received a new mount!")
		else
			-- Mount might already be owned or invalid ID
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Mount could not be unlocked (already owned or invalid).")
		end
	end
end


