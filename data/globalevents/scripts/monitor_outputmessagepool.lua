-- OutputMessagePool monitoring script for production servers
-- Run this on server startup and provides periodic monitoring functions

-- Configuration - adjust these values based on your server needs
local MONITOR_INTERVAL = 300000  -- 300 seconds = 5 minutes (adjust for your overhead tolerance)
local HIGH_BUFFERED_THRESHOLD = 100  -- Alert threshold for buffered protocols
local ALWAYS_LOG = true  -- Set to false to only log anomalies

-- Periodic monitoring function for server stats
function monitorServerStats()
    local bufferedCount = db.getBufferedProtocolsCount() or 0
    local poolSize = db.getPoolSize()
    local active = db.getActiveConnections()
    local available = db.getAvailableConnections()
    local transactionCount = db.getActiveTransactionCount() or 0
    local total = active + available

    -- Log if forced, or if there are anomalies
    -- Note: total can legitimately be < poolSize due to failed connections or on-demand creation
    local hasAnomaly = (available == 0 and active == 0 and poolSize > 0) or (bufferedCount > HIGH_BUFFERED_THRESHOLD)
    local shouldLog = ALWAYS_LOG or hasAnomaly or (bufferedCount > 0)

    if shouldLog then
        local status = (available > 0 or active > 0) and "OK" or "CRITICAL"
        print(string.format("[MONITOR] %s | Status: %s | Buffered: %d | DB Pool - Size: %d, Active: %d, Available: %d, Transactions: %d, Total: %d",
            os.date("%H:%M:%S"), status, bufferedCount, poolSize, active, available, transactionCount, total))
    end

    -- Check for critical connection issues (no connections available at all)
    if available == 0 and active == 0 and poolSize > 0 then
        print("[MONITOR] CRITICAL: No database connections available!")
        print(string.format("[MONITOR] Pool Size: %d, Active transactions: %d", poolSize, transactionCount))
        print("[MONITOR] This indicates a complete connection pool failure!")
    elseif available == 0 and transactionCount > 0 then
        print(string.format("[MONITOR] WARNING: No available connections but %d active transactions", transactionCount))
        print("[MONITOR] This may indicate a connection leak or high load.")
    end
    
    if bufferedCount > HIGH_BUFFERED_THRESHOLD then
        print("[MONITOR] WARNING: High buffered protocols count: " .. bufferedCount)
    end
end

-- Setup periodic monitoring (call this from your scripts)
function startPeriodicMonitoring()
    -- This function can be called to start periodic monitoring
    -- In TFS, you would typically call this from a globalevent or talkaction
    print("[MONITOR] Periodic monitoring started - call monitorServerStats() manually or schedule it")
end

-- Internal function to schedule next monitoring
local function scheduleNextMonitoring()
    addEvent(function()
        monitorServerStats()
        scheduleNextMonitoring() -- Schedule the next one
    end, MONITOR_INTERVAL)
end

function onStartup()
    print("[OUTPUTMESSAGEPOOL_MONITOR] Checking OutputMessagePool status...")

    -- Get current statistics
    local bufferedCount = db.getBufferedProtocolsCount()
    local poolSize = db.getPoolSize()
    local activeConnections = db.getActiveConnections()

    -- Log current state
    print(string.format("[OUTPUTMESSAGEPOOL_MONITOR] Buffered protocols: %d, DB Pool: %d active/%d total",
        bufferedCount, activeConnections, poolSize))

    -- Alert if buffered protocols count seems abnormal
    if bufferedCount > 50 then  -- Adjust threshold based on your server capacity
        print(string.format("[OUTPUTMESSAGEPOOL_WARNING] High buffered protocols count: %d - possible autosend backlog", bufferedCount))
    elseif bufferedCount < 0 then
        print("[OUTPUTMESSAGEPOOL_ERROR] Negative buffered protocols count - this should never happen!")
    else
        print("[OUTPUTMESSAGEPOOL_MONITOR] OutputMessagePool status: OK")
    end

    -- Start automatic periodic monitoring
    print("[OUTPUTMESSAGEPOOL_MONITOR] Starting automatic periodic monitoring (every minute)...")
    scheduleNextMonitoring()

    return true
end
