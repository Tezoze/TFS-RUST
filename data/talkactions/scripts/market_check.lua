-- Market diagnostic command for troubleshooting VPS market issues
function onSay(player, words, param)
	if not player:getGroup():getAccess() then
		return true
	end
	
	local playerId = player:getId()
	local depotId = player:getLastDepotId()
	
	player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "=== Market Diagnostic ===")
	player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Player ID: " .. playerId)
	player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Depot ID: " .. depotId)
	player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Premium: " .. (player:isPremium() and "Yes" or "No"))
	player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Bank Balance: " .. player:getBankBalance())
	player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Money: " .. player:getMoney())
	player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "In Market: " .. (player:isInMarket() and "Yes" or "No"))
	
	local depot = player:getDepotChest(depotId, false)
	if depot then
		player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Depot found: Yes")
		
		-- Count items in depot
		local itemCount = 0
		local wareIdItems = 0
		
		local function checkContainer(container)
			for i = 0, container:getSize() - 1 do
				local item = container:getItem(i)
				if item then
					itemCount = itemCount + 1
					
					local itemType = ItemType(item:getId())
					local wareId = itemType:getWareId()
					
					if item:getContainer() then
						checkContainer(item:getContainer())
					end
					
					if wareId > 0 then
						wareIdItems = wareIdItems + 1
					end
				end
			end
		end
		
		checkContainer(depot)
		
		player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Total depot items: " .. itemCount)
		player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Items with wareId: " .. wareIdItems)
		
		if wareIdItems == 0 then
			player:sendTextMessage(MESSAGE_STATUS_CONSOLE_ORANGE, "WARNING: No items have wareId! Market won't work.")
		end
	else
		player:sendTextMessage(MESSAGE_STATUS_CONSOLE_RED, "Depot found: NO! This will prevent market from working.")
	end
	
	player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "=== End Diagnostic ===")
	player:sendTextMessage(MESSAGE_STATUS_CONSOLE_ORANGE, "Check server.log and database for more details.")
	
	return false
end

