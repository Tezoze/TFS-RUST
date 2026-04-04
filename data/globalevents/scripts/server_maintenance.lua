function onStartup()
	-- Server launch time: January 2nd, 2026 at 6PM AEDT
	-- AEDT is UTC+10 during January (standard time)
	-- So 6PM AEDT = 8AM UTC on January 2, 2026
	local launchTime = os.time({
		year = 2026,
		month = 1,  -- January
		day = 2,
		hour = 8,   -- 8 AM UTC (6 PM AEDT)
		min = 0,
		sec = 0
	})

	local currentTime = os.time()

	if currentTime < launchTime then
		-- Server is not ready yet, close it
		local remainingTime = launchTime - currentTime
		local days = math.floor(remainingTime / 86400)
		local hours = math.floor((remainingTime % 86400) / 3600)
		local minutes = math.floor((remainingTime % 3600) / 60)

		local timeString = string.format("%d days, %d hours, %d minutes", days, hours, minutes)

		print(string.format(">> Server maintenance: Launch scheduled for January 2nd, 2026 at 6PM AEDT"))
		print(string.format(">> Time remaining: %s", timeString))
		print(">> Server maintenance mode scheduled - will set CLOSED state in 5 seconds...")

		-- Send maintenance message to any connected players
		local players = Game.getPlayers()
		for i, player in ipairs(players) do
			player:sendTextMessage(MESSAGE_STATUS_WARNING,
				"Server is under maintenance. Official launch: January 2nd, 2026 at 6PM AEDT.\n" ..
				string.format("Time remaining: %s", timeString))
		end

		-- Delay the state setting until server is fully online to ensure it takes effect
		addEvent(function()
			Game.setGameState(GAME_STATE_CLOSED)
			print(">> Server set to CLOSED state - blocking new logins but keeping server running")

			-- Also kick any players that managed to connect during startup
			local players = Game.getPlayers()
			local kickedCount = 0
			for i, player in ipairs(players) do
				if not player:hasFlag(PlayerFlag_CanAlwaysLogin) then
					player:kickPlayer(true)
					kickedCount = kickedCount + 1
				end
			end
			if kickedCount > 0 then
				print(">> Kicked " .. kickedCount .. " player(s) due to maintenance mode")
			end
		end, 5000) -- Wait 5 seconds for server to fully start

		return true
	end

	-- Server is ready, allow normal operation
	print(">> Server maintenance check passed - server is ready for launch!")
	return true
end
