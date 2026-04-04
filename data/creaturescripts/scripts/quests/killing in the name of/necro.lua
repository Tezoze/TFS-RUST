-- Adding the kill tracking table
local recent_kills = {}

function onKill(player, target)
	if target:isPlayer() or target:getMaster() then
		return true
	end

-- Adding duplicate prevention
	local targetId = target:getId()
	local playerGuid = player:getGuid()
	
	if not recent_kills[playerGuid] then
		recent_kills[playerGuid] = {}
	end
	
	if recent_kills[playerGuid][targetId] then
		return true
	end
	
	recent_kills[playerGuid][targetId] = true
	
	addEvent(function()
		if recent_kills[playerGuid] then
			recent_kills[playerGuid][targetId] = nil
		end
	end, 3000)

	if(isMonster(target)) then
	local killAmount = player:getStorageValue(Storage.KillingInTheNameOf.LugriNecromancerCount)
		if(string.lower(getCreatureName(target)) == "necromancer") and killAmount < 4000 and player:getStorageValue(Storage.KillingInTheNameOf.LugriNecromancers) == 1 then
			 player:setStorageValue(Storage.KillingInTheNameOf.LugriNecromancerCount, killAmount + 1)

			 elseif(string.lower(getCreatureName(target)) == "priestess") and killAmount < 4000 and player:getStorageValue(Storage.KillingInTheNameOf.LugriNecromancers) == 1 then
			 player:setStorageValue(Storage.KillingInTheNameOf.LugriNecromancerCount, killAmount + 1)

			 elseif(string.lower(getCreatureName(target)) == "blood priest") and killAmount < 4000 and player:getStorageValue(Storage.KillingInTheNameOf.LugriNecromancers) == 1 then
			 player:setStorageValue(Storage.KillingInTheNameOf.LugriNecromancerCount, killAmount + 1)

			 elseif(string.lower(getCreatureName(target)) == "blood hand") and killAmount < 4000 and player:getStorageValue(Storage.KillingInTheNameOf.LugriNecromancers) == 1 then
			 player:setStorageValue(Storage.KillingInTheNameOf.LugriNecromancerCount, killAmount + 1)

			 elseif(string.lower(getCreatureName(target)) == "shadow pupil") and killAmount < 4000 and player:getStorageValue(Storage.KillingInTheNameOf.LugriNecromancers) == 1 then
			 player:setStorageValue(Storage.KillingInTheNameOf.LugriNecromancerCount, killAmount + 1)
		end
	end

	return true
end
