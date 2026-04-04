local accountTitles = {
	[ACCOUNT_TYPE_NORMAL] = "Access Player",
	[ACCOUNT_TYPE_TUTOR] = "Tutor",
	[ACCOUNT_TYPE_SENIORTUTOR] = "Senior Tutor",
	[ACCOUNT_TYPE_GAMEMASTER] = "Gamemaster",
	[ACCOUNT_TYPE_GOD] = "God"
}

local commands = TalkAction("!commands", "/commands")

function commands.onSay(player)
	local isPlayerAccess = player:getGroup():getAccess()

	-- Hardcoded list of commands based on talkactions.xml
	local adminCommands = {
		"/attr", "/ban", "/ipban", "/unban", "/up", "/down", "!monitor", "/c", "/goto",
		"/owner", "/t", "/town", "/a", "/info", "/r", "/kick", "/openserver", "/closeserver",
		"/B", "/m", "/i", "/s", "/addtutor", "/removetutor", "/looktype", "/summon",
		"/chameleon", "/addskill", "/mccheck", "/ghost", "/clean", "/hide", "/reload",
		"/raid", "/roll", "/givepremiumpoints", "!z", "!x"
	}

	local playerCommands = {
		"!buyhouse", "!sellhouse", "!leavehouse",
		"!uptime", "!online", "!serverinfo"
	}

	local message = {"Available Commands:"}

	if isPlayerAccess then
		message[#message + 1] = "\nAdmin Commands:"
		for _, cmd in ipairs(adminCommands) do
			message[#message + 1] = cmd
		end
	end

	message[#message + 1] = "\nPlayer Commands:"
	for _, cmd in ipairs(playerCommands) do
		message[#message + 1] = cmd
	end

	player:showTextDialog(1950, table.concat(message, "\n"))
	return false
end

commands:register()
