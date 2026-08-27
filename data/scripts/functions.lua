local logFormat = "[%s] %s %s"

function logCommand(player, words, param)
	tfs.appendLog(player:getName() .. " commands.log", logFormat:format(os.date("%d/%m/%Y %H:%M"), words, param):trim() .. "\n")
end

function Player:addPartyCondition(combat, variant, condition, baseMana)
	local party = self:getParty()
	if not party then
		self:sendCancelMessage(RETURNVALUE_NOPARTYMEMBERSINRANGE)
		self:getPosition():sendMagicEffect(CONST_ME_POFF)
		return false
	end

	local positions = combat:getPositions(self, variant)
	local members = party:getMembers()
	members[#members + 1] = party:getLeader()

	local affectedMembers = {}
	for _, member in ipairs(members) do
		local memberPosition = member:getPosition()
		for _, position in ipairs(positions) do
			if memberPosition == position then
				affectedMembers[#affectedMembers + 1] = member
			end
		end
	end

	if #affectedMembers <= 1 then
		self:sendCancelMessage(RETURNVALUE_NOPARTYMEMBERSINRANGE)
		self:getPosition():sendMagicEffect(CONST_ME_POFF)
		return false
	end

	local mana = math.ceil(#affectedMembers * math.pow(0.9, #affectedMembers - 1) * baseMana)
	if self:getMana() < mana then
		self:sendCancelMessage(RETURNVALUE_NOTENOUGHMANA)
		self:getPosition():sendMagicEffect(CONST_ME_POFF)
		return false
	end

	self:addMana(-mana)
	self:addManaSpent(mana)

	for _, member in ipairs(affectedMembers) do
		member:addCondition(condition)
	end

	for _, position in ipairs(positions) do
		position:sendMagicEffect(CONST_ME_MAGIC_BLUE)
	end
	return true
end

function Creature:addAttributeCondition(parameters)
	local condition = Condition(CONDITION_ATTRIBUTES)
	for _, parameter in ipairs(parameters) do
		if parameter.key and parameter.value then
			condition:setParameter(parameter.key, parameter.value)
		end
	end

	self:addCondition(condition)
end
