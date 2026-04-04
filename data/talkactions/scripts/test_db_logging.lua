function onSay(player, words, param)
	if not player:getGroup():getAccess() then
		return true
	end

	player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "[DB LOG TEST] Starting database error logging tests...")

	-- Test 1: Invalid table name
	player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "[DB LOG TEST] Testing invalid table query...")
	local result = db.storeQuery("SELECT * FROM nonexistent_table_12345")
	if not result then
		player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "[DB LOG TEST] ✓ Invalid table query failed as expected")
	end

	-- Test 2: Invalid SQL syntax
	player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "[DB LOG TEST] Testing invalid SQL syntax...")
	local success = db.query("INVALID SQL SYNTAX HERE")
	if not success then
		player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "[DB LOG TEST] ✓ Invalid SQL syntax failed as expected")
	end

	-- Test 3: Valid query to ensure normal operation still works
	player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "[DB LOG TEST] Testing valid query...")
	local success = db.query("SELECT 1")
	if success then
		player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "[DB LOG TEST] ✓ Valid query succeeded")
	else
		player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "[DB LOG TEST] ✗ Valid query failed unexpectedly")
	end

	-- Test 4: Transaction with invalid operation
	player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "[DB LOG TEST] Testing transaction with invalid operation...")
	if db.beginTransaction() then
		local success1 = db.query("UPDATE players SET name = 'test' WHERE id = -999")
		local success2 = db.query("INVALID QUERY IN TRANSACTION")
		if db.rollback() then
			player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "[DB LOG TEST] ✓ Transaction rollback succeeded")
		end
	else
		player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "[DB LOG TEST] ✗ Failed to begin transaction")
	end

	player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "[DB LOG TEST] Database error logging tests completed!")
	player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "[DB LOG TEST] Check data/logs/database_errors.log for logged errors")

	return false
end
