local soulCondition = Condition(CONDITION_SOUL, CONDITIONID_DEFAULT)
soulCondition:setTicks(4 * 60 * 1000)
soulCondition:setParameter(CONDITION_PARAM_SOULGAIN, 1)

-- Double XP Event Storage Keys (must match double_exp.lua)
local STORAGE_DOUBLE_EXP = 39901
local STORAGE_DOUBLE_EXP_END = 39902

local function isDoubleExpActive()
	local enabled = Game.getStorageValue(STORAGE_DOUBLE_EXP)
	if enabled ~= 1 then
		return false
	end
	
	local endTime = Game.getStorageValue(STORAGE_DOUBLE_EXP_END)
	if endTime <= 0 then
		return true
	end
	
	return os.time() < endTime
end

local event = Event()

function event.onGainExperience(player, source, exp, rawExp)
	if not source or source:isPlayer() then return exp end

	-- Soul regeneration
	local vocation = player:getVocation()
	if player:getSoul() < vocation:getMaxSoul() and exp >= player:getLevel() then
		soulCondition:setParameter(CONDITION_PARAM_SOULTICKS, vocation:getSoulGainTicks() * 1000)
		player:addCondition(soulCondition)
	end

	-- Apply experience stage multiplier
	exp = exp * Game.getExperienceStage(player:getLevel())

	-- Stamina modifier
	player:updateStamina()

	-- Experience Rates
	local staminaRate = player:getExperienceRate(ExperienceRateType.STAMINA)
	if staminaRate ~= 100 then exp = exp * staminaRate / 100 end

	local baseRate = player:getExperienceRate(ExperienceRateType.BASE)
	if baseRate ~= 100 then exp = exp * baseRate / 100 end

	local lowLevelRate = player:getExperienceRate(ExperienceRateType.LOW_LEVEL)
	if lowLevelRate ~= 100 then exp = exp * lowLevelRate / 100 end

	local bonusRate = player:getExperienceRate(ExperienceRateType.BONUS)
	if bonusRate ~= 100 then exp = exp * bonusRate / 100 end

	-- Double XP Event bonus (applied last, doubles final exp)
	if isDoubleExpActive() then
		exp = exp * 2
	end

	return exp
end

event:register()
