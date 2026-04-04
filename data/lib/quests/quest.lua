-- Basic quest system for TFS compatibility
-- This file provides basic quest functionality
-- Avoids conflicts with core/quests.lua by using different names

LegacyQuest = {}

-- Basic quest class
function LegacyQuest:new(name)
	local quest = {
		name = name,
		missions = {},
		storage = 0
	}
	setmetatable(quest, self)
	self.__index = self
	return quest
end

function LegacyQuest:addMission(mission)
	table.insert(self.missions, mission)
end

function LegacyQuest:getName()
	return self.name
end

function LegacyQuest:getMissions()
	return self.missions
end

-- Mission class
LegacyMission = {}

function LegacyMission:new(name, storage, startValue, endValue)
	local mission = {
		name = name,
		storage = storage,
		startValue = startValue,
		endValue = endValue
	}
	setmetatable(mission, self)
	self.__index = self
	return mission
end

function LegacyMission:getName()
	return self.name
end

function LegacyMission:getStorage()
	return self.storage
end

function LegacyMission:getStartValue()
	return self.startValue
end

function LegacyMission:getEndValue()
	return self.endValue
end

-- Export to global scope for compatibility - only if not already defined
if not _G.Quest then _G.Quest = LegacyQuest end
if not _G.Mission then _G.Mission = LegacyMission end

-- Load existing quest files
dofile('data/lib/quests/demon_oak.lua')
dofile('data/lib/quests/killing_in_the_name_of.lua')
dofile('data/lib/quests/svargrond_arena.lua')

