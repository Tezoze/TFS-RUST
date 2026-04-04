function Party:onJoin(player)
	if hasEventCallback(EVENT_CALLBACK_ONJOIN) then
		return EventCallback(EVENT_CALLBACK_ONJOIN, self, player)
	else
		return true
	end
end

function Party:onLeave(player)
	if hasEventCallback(EVENT_CALLBACK_ONLEAVE) then
		return EventCallback(EVENT_CALLBACK_ONLEAVE, self, player)
	else
		return true
	end
end

function Party:onDisband()
	if hasEventCallback(EVENT_CALLBACK_ONDISBAND) then
		return EventCallback(EVENT_CALLBACK_ONDISBAND, self)
	else
		return true
	end
end

function Party:onShareExperience(exp)
	local sharedExperienceMultiplier = 120 --120% (1.20x)
	local rawExp = exp

	-- Count unique vocations in the party (using base vocations to group promoted forms)
	local vocations = {}
	local leader = self:getLeader()
	if leader then
		local baseVocation = leader:getVocation():getBase()
		vocations[baseVocation:getId()] = true
	end
	
	for _, member in ipairs(self:getMembers()) do
		local baseVocation = member:getVocation():getBase()
		vocations[baseVocation:getId()] = true
	end
	
	-- Count unique vocations, cap at 4 for bonus calculation
	local uniqueVocations = 0
	for _ in pairs(vocations) do
		uniqueVocations = uniqueVocations + 1
	end
	uniqueVocations = math.min(uniqueVocations, 4)

	-- Count party members (leader + members), cap at 4 for bonus calculation
	local partySize = math.min(#self:getMembers() + 1, 4)

	-- Calculate vocation-based bonus (full bonus)
	-- 2 vocations: +30%
	-- 3 vocations: +60%
	-- 4 vocations: +100%
	local vocationBonus = 0
	if uniqueVocations > 1 then
		vocationBonus = uniqueVocations * (5 * (uniqueVocations - 1) + 10)
	end

	-- Calculate player-based bonus (50% of vocation formula)
	-- 2 players: +15%
	-- 3 players: +30%
	-- 4 players: +50%
	local playerBonus = 0
	if partySize > 1 then
		playerBonus = (partySize * (5 * (partySize - 1) + 10)) * 0.5
	end

	-- Use whichever bonus is higher (they don't stack)
	local finalBonus = math.max(vocationBonus, playerBonus)
	sharedExperienceMultiplier = 100 + finalBonus

	-- Apply the multiplier to the raw experience and return the final amount
	local result = sharedExperienceMultiplier
	if hasEventCallback(EVENT_CALLBACK_ONSHAREEXPERIENCE) then
		result = EventCallback(EVENT_CALLBACK_ONSHAREEXPERIENCE, self, sharedExperienceMultiplier, rawExp)
	end
	
	return rawExp * (result / 100)
end
