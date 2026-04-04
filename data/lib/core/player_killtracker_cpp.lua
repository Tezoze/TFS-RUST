-- Alternative implementation using C++ sendKillTracker method
-- Use this if the NetworkMessage Lua approach doesn't work

function Player.updateKillTracker(self, monster, corpse)
	print("[DEBUG CPP] updateKillTracker called for player: " .. self:getName())
	
	local monsterType = monster:getType()
	if not monsterType then
		print("[DEBUG CPP] No monster type found!")
		return false
	end
	
	-- Get monster info
	local monsterName = monster:getName()
	local monsterOutfit = monsterType:getOutfit()
	
	-- Build loot items table
	local lootItems = {}
	local corpseSize = corpse:getSize()
	print("[DEBUG CPP] Corpse size: " .. corpseSize)
	
	for index = corpseSize - 1, 0, -1 do
		local item = corpse:getItem(index)
		if item then
			table.insert(lootItems, {
				id = item:getId(),
				name = item:getName(),
				count = item:getItemCount()
			})
			print("[DEBUG CPP] Added item: " .. item:getName() .. " (count: " .. item:getItemCount() .. ")")
		end
	end
	
	print("[DEBUG CPP] Total loot items: " .. #lootItems)
	
	-- Send to player and party using C++ method
	local party = self:getParty()
	if party then
		local members = party:getMembers()
		members[#members + 1] = party:getLeader()
		print("[DEBUG CPP] Sending to party (" .. #members .. " members)")
		
		for _, member in ipairs(members) do
			member:sendKillTracker(monsterName, monsterOutfit, lootItems)
		end
	else
		print("[DEBUG CPP] Sending to solo player")
		self:sendKillTracker(monsterName, monsterOutfit, lootItems)
	end
	
	print("[DEBUG CPP] updateKillTracker completed successfully")
	return true
end






























