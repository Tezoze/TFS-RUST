function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	-- Check if a key is being used on the door
	if item.itemid >= 2086 and item.itemid <= 2089 and item:getActionId() == 3802 then
		-- Key used on door
		if target and target.getActionId then
			if target:getActionId() == 3802 then
				-- Door is locked, unlock it and open
				target:setActionId(3803) -- Mark as unlocked
				target:removeAttribute(ITEM_ATTRIBUTE_DESCRIPTION) -- Remove locked description
				target:transform(target.itemid + 1) -- Open the door (most doors follow closed + 1 = open pattern)
			elseif target:getActionId() == 3803 then
				-- Door is unlocked, lock it and close
				target:setActionId(3802) -- Mark as locked
				target:setAttribute(ITEM_ATTRIBUTE_DESCRIPTION, "It is locked.") -- Add locked description
				target:transform(target.itemid - 1) -- Close the door (most doors follow open - 1 = closed pattern)
			end
			return true
		end
	end

	-- Handle door interactions
	if item:getActionId() == 3802 then
		-- Door is locked - check for key with action ID 3802
		local hasKey = false
		-- Check backpack slot (slot 3) where keys are usually stored
		local backpack = player:getSlotItem(3)
		if backpack and backpack:isContainer() then
			for i = 0, backpack:getSize() - 1 do
				local backpackItem = backpack:getItem(i)
				if backpackItem and backpackItem:getActionId() == 3802 then
					hasKey = true
					break
				end
			end
		end

		if hasKey then
			-- Has key, unlock and open the door
			item:setActionId(3803) -- Mark as unlocked
			item:removeAttribute(ITEM_ATTRIBUTE_DESCRIPTION) -- Remove locked description
			item:transform(item.itemid + 1) -- Open the door
		else
			-- No key, ensure locked description is shown
			item:setAttribute(ITEM_ATTRIBUTE_DESCRIPTION, "It is locked.")
			player:sendTextMessage(MESSAGE_EVENT_DEFAULT, "The door is locked.")
		end
	elseif item:getActionId() == 3803 then
		-- Door is unlocked, just open it
		item:transform(item.itemid + 1) -- Open the door
	else
		-- Regular door without action ID, just toggle open/close
		local isEven = (item.itemid % 2) == 0
		if isEven then
			-- Even ID (open door), close it
			item:transform(item.itemid - 1)
		else
			-- Odd ID (closed door), open it
			item:transform(item.itemid + 1)
		end
	end
	return true
end
