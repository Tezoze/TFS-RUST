function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	-- Check if a key is being used on the door
	if item.itemid >= 2090 and item.itemid <= 2095 then
		local keyAID = item:getActionId()
		-- Key used on door
		if target and target.getActionId then
			local targetAID = target:getActionId()
			local unlockAID = 3805
			if (targetAID == 3801 and keyAID == 3801) then
				-- Door is locked and key matches, unlock it and open
				target:setActionId(unlockAID) -- Mark as unlocked
				target:removeAttribute(ITEM_ATTRIBUTE_DESCRIPTION) -- Remove locked description
				target:transform(target.itemid + 1) -- Open the door
			elseif targetAID == 3805 then
				-- Door is unlocked, check if key matches for locking
				local lockAID = 3801
				local requiredKeyAID = 3801 -- Crystal keys are always AID 3801
				if keyAID == requiredKeyAID then
					target:setActionId(lockAID) -- Mark as locked
					target:setAttribute(ITEM_ATTRIBUTE_DESCRIPTION, "It is locked.") -- Add locked description
					local isEven = (target.itemid % 2) == 0
					if not isEven then
						-- Door is open (odd ID), close it
						target:transform(target.itemid - 1)
					end
					-- If door is already closed, don't transform
				end
			end
			return true
		end
	end

	-- Handle door interactions
	local itemAID = item:getActionId()
	if itemAID == 3801 then
		-- Door is locked - check for matching crystal key (AID 3801)
		local hasKey = false
		local requiredKeyAID = 3801 -- Crystal keys are always AID 3801
		-- Check backpack slot (slot 3) where keys are usually stored
		local backpack = player:getSlotItem(3)
		if backpack and backpack:isContainer() then
			for i = 0, backpack:getSize() - 1 do
				local backpackItem = backpack:getItem(i)
				if backpackItem and backpackItem:getActionId() == requiredKeyAID then
					hasKey = true
					break
				end
			end
		end

		if hasKey then
			-- Has matching crystal key, unlock and open the door
			local unlockAID = 3805
			item:setActionId(unlockAID) -- Mark as unlocked
			item:removeAttribute(ITEM_ATTRIBUTE_DESCRIPTION) -- Remove locked description
			item:transform(item.itemid + 1) -- Open the door
		else
			-- No matching crystal key, ensure locked description is shown
			item:setAttribute(ITEM_ATTRIBUTE_DESCRIPTION, "It is locked.")
			player:sendTextMessage(MESSAGE_EVENT_DEFAULT, "The door is locked.")
		end
	elseif itemAID == 3805 then
		-- Door is unlocked, just toggle open/close normally
		local isEven = (item.itemid % 2) == 0
		if isEven then
			-- Even ID (closed door), open it
			item:transform(item.itemid + 1)
		else
			-- Odd ID (open door), close it
			item:transform(item.itemid - 1)
		end
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
