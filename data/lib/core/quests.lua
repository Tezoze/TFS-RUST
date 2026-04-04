local quests = {}
local missions = {}

Quest = {}
Quest.__index = Quest

function Quest:register()
	self.id = #quests + 1
	for _, mission in pairs(self.missions) do
		mission.id = #missions + 1
		mission.questId = self.id
		-- Preserve custom isStarted/isCompleted functions before applying metatable
		local customIsStarted = mission.isStarted
		local customIsCompleted = mission.isCompleted
		-- Delete raw fields so they don't shadow metatable methods
		mission.isStarted = nil
		mission.isCompleted = nil
		missions[mission.id] = setmetatable(mission, Mission)
		-- Store as customIsStarted/customIsCompleted for metatable methods to use
		if type(customIsStarted) == "function" then
			mission.customIsStarted = customIsStarted
		end
		if type(customIsCompleted) == "function" then
			mission.customIsCompleted = customIsCompleted
		end
	end

	quests[self.id] = self
	return true
end

function Quest:isStarted(player)
	return player:getStorageValue(self.storageId, 0) >= self.storageValue
end

function Quest:isCompleted(player)
	for _, mission in pairs(self.missions) do
		if not mission:isCompleted(player) then return false end
	end
	return true
end

function Quest:getMissions(player)
	local playerMissions = {}
	for _, mission in pairs(self.missions) do
		if mission:isStarted(player) then playerMissions[#playerMissions + 1] = mission end
	end
	return playerMissions
end

Mission = {}
Mission.__index = Mission

function Mission:isStarted(player)
	-- Support custom isStarted function
	if type(self.customIsStarted) == "function" then
		return self.customIsStarted(player)
	end
	
	if not player or not player.getStorageValue then return false end
	local value = player:getStorageValue(self.storageId, 0)
	local startVal = tonumber(self.startValue) or 0
	if value >= startVal then
		if self.ignoreEndValue then
			return true
		else
			local endVal = tonumber(self.endValue) or 999999
			return value <= endVal
		end
	end
	return false
end

function Mission:isCompleted(player)
	-- Support custom isCompleted function
	if type(self.customIsCompleted) == "function" then
		return self.customIsCompleted(player)
	end
	
	if not player or not player.getStorageValue then return false end
	local value = player:getStorageValue(self.storageId, 0)
	if self.ignoreEndValue then
		local endVal = tonumber(self.endValue) or 999999
		return value >= endVal
	end
	local endVal = tonumber(self.endValue) or 999999
	return value == endVal
end

function Mission:getName(player)
	if self:isCompleted(player) then return string.format("%s (Completed)", self.name) end
	return self.name
end

function Mission:getDescription(player)
	local descriptionType = type(self.description)
	if descriptionType == "function" then return self.description(player) end

	if not player or not player.getStorageValue then
		return "An error has occurred, please contact a gamemaster."
	end
	local value = player:getStorageValue(self.storageId, 0)
	if descriptionType == "string" then
		local description = self.description:gsub("|STATE|", value)
		description = self.description:gsub("\\n", "\n")
		return description
	end

	if descriptionType == "table" then
		if self.ignoreEndValue then
			local endVal = tonumber(self.endValue) or 999999
			local startVal = tonumber(self.startValue) or 0
			for current = endVal, startVal, -1 do
				if value >= current then return self.description[current] end
			end
		else
			local endVal = tonumber(self.endValue) or 999999
			local startVal = tonumber(self.startValue) or 0
			for current = endVal, startVal, -1 do
				if value == current then return self.description[current] end
			end
		end
	end

	return "An error has occurred, please contact a gamemaster."
end

function Game.getQuests() return quests end
function Game.getMissions() return missions end

function Game.getQuestById(id) return quests[id] end
function Game.getMissionById(id) return missions[id] end

function Game.clearQuests()
	quests = {}
	missions = {}
	return true
end

function Game.createQuest(name, quest)
	-- Removed isScriptsInterface() check for TFS 1.5 compatibility during startup quest loading

	if type(quest) == "table" then
		setmetatable(quest, Quest)
		quest.id = -1
		quest.name = name
		if type(quest.missions) ~= "table" then quest.missions = {} end

		return quest
	end

	quest = setmetatable({}, Quest)
	quest.id = -1
	quest.name = name
	quest.storageId = 0
	quest.storageValue = 0
	quest.missions = {}
	return quest
end

function Game.isQuestStorage(key, value, oldValue)
	-- Check if this is a quest start storage
	for _, quest in pairs(quests) do
		if quest.storageId == key and quest.storageValue == value then 
			return true 
		end

		-- Check if this is a mission storage
		for _, mission in pairs(quest.missions) do
			if mission.storageId == key then
				local startVal = tonumber(mission.startValue) or 0
				local endVal = tonumber(mission.endValue) or 999999
				
				-- Return true if value is in mission range and transitioned from outside the range
				if value >= startVal and value <= endVal then
					-- Trigger if: no oldValue, oldValue was outside range, or value changed
					return not oldValue or oldValue < startVal or oldValue > endVal or oldValue ~= value
				end
			end
		end
	end
	return false
end

function Player:getQuests()
	local playerQuests = {}
	for _, quest in pairs(quests) do
		if quest:isStarted(self) then playerQuests[#playerQuests + 1] = quest end
	end
	return playerQuests
end

function Player:sendQuestLog()
	local msg = NetworkMessage()
	msg:addByte(0xF0)
	local quests = self:getQuests()
	msg:addU16(#quests)

	for _, quest in pairs(quests) do
		msg:addU16(quest.id)
		msg:addString(quest.name)
		msg:addByte(quest:isCompleted(self) and 1 or 0)
	end

	msg:sendToPlayer(self)
	return true
end

function Player:sendQuestLine(quest)
	if not quest or not quest.id then
		return false
	end

	local msg = NetworkMessage()
	msg:addByte(0xF1)
	msg:addU16(quest.id)
	local missions = quest:getMissions(self)
	msg:addByte(#missions)

	for _, mission in pairs(missions) do
		msg:addString(mission:getName(self))
		msg:addString(mission:getDescription(self))
	end

	msg:sendToPlayer(self)
	return true
end

function Player:sendQuestLineById(questId)
	local quest = Game.getQuestById(questId)
	if not quest then
		return false
	end
	return self:sendQuestLine(quest)
end

-- Show quest advancement message to player
function Game.sendQuestUpdateMessage(player)
	if player and player.sendTextMessage then
		player:sendTextMessage(MESSAGE_INFO_DESCR, "Your quest log has been updated.")
	end
end
