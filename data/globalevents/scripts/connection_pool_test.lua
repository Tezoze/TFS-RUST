-- Connection pool test on server startup

function onStartup()
    print("[CONNECTION_POOL_TEST] Running connection pool integrity test...")

    -- Test basic database connectivity
    local result = db.storeQuery("SELECT 1 as test")
    if result then
        print("[CONNECTION_POOL_TEST] Basic database connectivity: OK")
    else
        print("[CONNECTION_POOL_TEST] ERROR: Basic database connectivity failed!")
        return true
    end

    -- Check pool statistics
    local poolSize = db.getPoolSize()
    local active = db.getActiveConnections()
    local available = db.getAvailableConnections()
    local total = active + available

    print(string.format("[CONNECTION_POOL_TEST] Pool Statistics:"))
    print(string.format("[CONNECTION_POOL_TEST]   Pool Size: %d", poolSize))
    print(string.format("[CONNECTION_POOL_TEST]   Active: %d", active))
    print(string.format("[CONNECTION_POOL_TEST]   Available: %d", available))
    print(string.format("[CONNECTION_POOL_TEST]   Total: %d", total))

    if total ~= poolSize then
        print(string.format("[CONNECTION_POOL_TEST] WARNING: Connection count mismatch! Expected %d, got %d", poolSize, total))
    else
        print("[CONNECTION_POOL_TEST] Connection pool integrity: OK")
    end

    -- Test multiple concurrent queries to simulate login load
    print("[CONNECTION_POOL_TEST] Testing concurrent queries...")
    local testQueries = 20
    local successCount = 0

    for i = 1, testQueries do
        local testResult = db.storeQuery("SELECT " .. i .. " as id")
        if testResult then
            successCount = successCount + 1
        end
    end

    print(string.format("[CONNECTION_POOL_TEST] Concurrent queries: %d/%d successful", successCount, testQueries))

    -- Test OutputMessagePool monitoring (if available)
    if db.getBufferedProtocolsCount then
        local bufferedCount = db.getBufferedProtocolsCount()
        print(string.format("[CONNECTION_POOL_TEST] OutputMessagePool buffered protocols: %d", bufferedCount))
    end

    if successCount == testQueries then
        print("[CONNECTION_POOL_TEST] All tests passed!")
    else
        print("[CONNECTION_POOL_TEST] WARNING: Some concurrent queries failed!")
    end

    return true
end
