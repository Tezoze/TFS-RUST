-- /give command for gods to grant experience, skills, or magic levels to players
-- Usage: /give <type> <amount> <playername>
-- Examples:
--   /give magic 1 Tezoze      - Gives 1 magic level
--   /give shield 1 Tezoze     - Gives 1 shielding level
--   /give experience 4200 Tezoze - Gives 4200 experience

local skillMap = {
	["fist"] = SKILL_FIST,
	["club"] = SKILL_CLUB,
	["sword"] = SKILL_SWORD,
	["axe"] = SKILL_AXE,
	["distance"] = SKILL_DISTANCE,
	["dist"] = SKILL_DISTANCE,
	["shield"] = SKILL_SHIELD,
	["shielding"] = SKILL_SHIELD,
	["fishing"] = SKILL_FISHING,
	["fish"] = SKILL_FISHING,
	["magic"] = SKILL_MAGLEVEL,
	["maglevel"] = SKILL_MAGLEVEL,
	["ml"] = SKILL_MAGLEVEL
}

local give = TalkAction("/give")

function give.onSay(player, words, param)
	-- Check if player is a god
	if player:getAccountType() < ACCOUNT_TYPE_GOD then
		player:sendCancelMessage("You cannot use this command.")
		return false
	end

	-- Parse parameters: /give <type> <amount> <playername>
	local split = param:splitTrimmed(" ")
	if #split < 3 then
		player:sendCancelMessage("Usage: /give <type> <amount> <playername>")
		player:sendCancelMessage("Types: magic, shield, sword, axe, club, fist, distance, fishing, experience")
		return false
	end

	local giveType = split[1]:lower()
	local amount = tonumber(split[2])
	
	-- Player name can have spaces, so join remaining parts
	local playerName = table.concat(split, " ", 3)

	if not amount or amount <= 0 then
		player:sendCancelMessage("Amount must be a positive number.")
		return false
	end

	-- Find the target player
	local target = Player(playerName)
	if not target then
		player:sendCancelMessage("Player '" .. playerName .. "' is not online.")
		return false
	end

	-- Handle experience separately
	if giveType == "experience" or giveType == "exp" then
		target:addExperience(amount, true)
		player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, 
			"Gave " .. amount .. " experience to " .. target:getName() .. ".")
		target:sendTextMessage(MESSAGE_STATUS_CONSOLE_ORANGE, 
			"You have received " .. amount .. " experience from " .. player:getName() .. ".")
		return false
	end

	-- Handle skills and magic level
	local skillId = skillMap[giveType]
	if not skillId then
		player:sendCancelMessage("Unknown type: " .. giveType)
		player:sendCancelMessage("Valid types: magic, shield, sword, axe, club, fist, distance, fishing, experience")
		return false
	end

	-- Add the skill levels
	for i = 1, amount do
		target:addSkill(skillId, 1)
	end

	-- Get skill name for message
	local skillName = giveType
	if skillId == SKILL_MAGLEVEL then
		skillName = "magic level"
	elseif skillId == SKILL_SHIELD then
		skillName = "shielding"
	elseif skillId == SKILL_DISTANCE then
		skillName = "distance fighting"
	elseif skillId == SKILL_FISHING then
		skillName = "fishing"
	end

	player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, 
		"Gave " .. amount .. " " .. skillName .. " to " .. target:getName() .. ".")
	target:sendTextMessage(MESSAGE_STATUS_CONSOLE_ORANGE, 
		"You have received " .. amount .. " " .. skillName .. " from " .. player:getName() .. ".")

	return false
end

give:separator(" ")
give:register()
