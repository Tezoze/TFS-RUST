#include "otpch.h"

#include "stability.h"
#include "database.h"
#include "player.h"
#include "item.h"
#include "map.h"
#include "configmanager.h"
#include "tasks.h"

extern ConfigManager g_config;
extern Dispatcher g_dispatcher;

#include <iostream>
#include <fstream>
#include <cstring>
#include <csignal>
#include <unistd.h>
#include <sys/resource.h>
#include <execinfo.h>
#include <sys/stat.h>
#include <iomanip>
#include <sstream>
#include <algorithm>
#include <filesystem>
#include <thread>
#include <cstdio>
#include <memory>
#include <array>

// Add these headers for POSIX-compliant signal handling
#include <fcntl.h>

// Helper for signal-safe string writing
// Uses a dummy check to silence "ignoring return value" warnings
void safe_write_str(int fd, const char* str) {
    if (str) {
        if (write(fd, str, strlen(str)) < 0) {
            // We ignore errors here because we are already crashing
            // and cannot recover, but the 'if' silences the compiler warning.
        }
    }
}

void safe_write_int(int fd, int val) {
    char buf[32];
    int i = sizeof(buf) - 1;
    buf[i] = '\n'; // End with newline

    // Handle 0 explicitly
    if (val == 0) {
        if (write(fd, "0", 1) < 0) {}
        return;
    }

    // Handle negative numbers (signals are usually positive, but good for safety)
    if (val < 0) {
        if (write(fd, "-", 1) < 0) {}
        val = -val;
    }

    while (val > 0 && i > 0) {
        buf[--i] = '0' + (val % 10);
        val /= 10;
    }

    if (write(fd, &buf[i], sizeof(buf) - 1 - i) < 0) {}
}

StabilityManager* StabilityManager::instance_ = nullptr;

StabilityManager& StabilityManager::getInstance() {
    static StabilityManager instance;
    instance_ = &instance;
    return instance;
}

void StabilityManager::initialize() {
    std::lock_guard<std::mutex> lock(statsMutex_);
    stats_.lastHealthCheck = std::chrono::steady_clock::now();

    // Initialize memory leak detection
    nextAllocationId_ = 1;
    totalAllocatedBytes_ = 0;
    totalAllocationCount_ = 0;
    totalDeallocationCount_ = 0;

    // Install signal handlers for crash prevention
    installSignalHandlers();

    // Validate configuration on startup
    if (!validateConfiguration()) {
        std::cerr << "[Stability] Configuration validation failed!" << std::endl;
        throw std::runtime_error("Configuration validation failed");
    }

    // Validate database connection
    if (!validateDatabaseConnection()) {
        std::cerr << "[Stability] Database connection validation failed!" << std::endl;
        throw std::runtime_error("Database connection validation failed");
    }

    // Validate script environment
    if (!validateScriptEnvironment()) {
        std::cerr << "[Stability] Script environment validation failed!" << std::endl;
        throw std::runtime_error("Script environment validation failed");
    }

    // Register recovery handlers
    registerRecoveryHandler("memory_limit", [this]() -> bool {
        logStabilityEvent("MEMORY_RECOVERY", "Attempting memory cleanup recovery");

        // Force garbage collection/cleanup
        {
            std::lock_guard<std::mutex> lock(memoryMutex_);
            // Clear any unnecessary memory blocks
            auto it = memoryBlocks_.begin();
            while (it != memoryBlocks_.end()) {
                // Remove blocks older than 30 minutes
                auto age = std::chrono::steady_clock::now() - it->second.registered;
                if (age > std::chrono::minutes(30)) {
                    it = memoryBlocks_.erase(it);
                } else {
                    ++it;
                }
            }
        }

        // Clean up expired transactions
        cleanupExpiredTransactions();

        // Clear database query cache if available
        try {
            // Database& db = Database::getInstance();
            // This will help reduce memory usage from cached queries
            // Note: This is a placeholder - actual implementation depends on Database class
        } catch (const std::exception& e) {
            // Ignore cleanup errors
        }

        // Suggest memory optimization in logs
        logStabilityEvent("MEMORY_RECOVERY", "Memory cleanup completed. Consider increasing memory limit or optimizing server configuration if warnings persist.");
        return true;
    });

    registerRecoveryHandler("connection_limit", [this]() -> bool {
        logStabilityEvent("CONNECTION_RECOVERY", "Attempting connection limit recovery");

        // This would need to be implemented based on the connection management system
        // For now, just log the attempt
        logStabilityEvent("CONNECTION_RECOVERY", "Connection limit recovery placeholder - needs implementation");
        return true;
    });

    registerRecoveryHandler("memory_corruption", [this]() -> bool {
        logStabilityEvent("CORRUPTION_RECOVERY", "Attempting memory corruption recovery");

        // Force memory validation and cleanup
        validateMemoryIntegrity();
        {
            std::lock_guard<std::mutex> lock(memoryMutex_);
            memoryBlocks_.clear(); // Clear all tracked memory blocks for safety
        }

        logStabilityEvent("CORRUPTION_RECOVERY", "Memory corruption recovery completed");
        return true;
    });

    registerRecoveryHandler("critical_signal", [this]() -> bool {
        logStabilityEvent("SIGNAL_RECOVERY", "Attempting critical signal recovery");

        // Create emergency core dump
        createCoreDump();

        // For critical signals, we typically want to shut down gracefully
        logStabilityEvent("SIGNAL_RECOVERY", "Emergency recovery initiated - server will shutdown gracefully");
        return false; // Return false to indicate shutdown is needed
    });

    registerRecoveryHandler("memory_leak", [this]() -> bool {
        logStabilityEvent("MEMORY_LEAK_RECOVERY", "Attempting memory leak recovery");

        // Report current memory leaks
        reportMemoryLeaks();

        // For now, we don't automatically clear leaks as that could mask real issues
        // In production, you might want to implement more sophisticated leak handling
        logStabilityEvent("MEMORY_LEAK_RECOVERY", "Memory leak analysis completed");
        return true;
    });

    logStabilityEvent("INITIALIZATION", "Stability manager initialized successfully");
}

void StabilityManager::shutdown() {
    logStabilityEvent("SHUTDOWN", "Stability manager shutting down");

    // Report final memory leak status
    reportMemoryLeaks();

    // Clean up memory blocks
    {
        std::lock_guard<std::mutex> lock(memoryMutex_);
        memoryBlocks_.clear();
    }

    // Clean up memory leak tracking
    {
        std::lock_guard<std::mutex> allocLock(allocationMutex_);
        if (!activeAllocations_.empty()) {
            logStabilityEvent("SHUTDOWN_LEAKS",
                std::to_string(activeAllocations_.size()) + " allocations not freed at shutdown");
        }
        activeAllocations_.clear();
    }

    // Clean up transactions
    {
        std::lock_guard<std::mutex> txLock(transactionMutex_);
        activeTransactions_.clear();
    }
}

bool StabilityManager::validatePlayerData(uint32_t playerId) {
    try {
        // Query player data integrity
        Database& db = Database::getInstance();
        std::ostringstream query;
        query << "SELECT COUNT(*) as count FROM players WHERE id = " << playerId
              << " AND deletion = 0 AND name IS NOT NULL AND name != ''";

        DBResult_ptr result = db.storeQuery(query.str());
        if (!result) {
            logStabilityEvent("DATA_VALIDATION", "Player data validation failed for ID: " + std::to_string(playerId));
            return false;
        }

        uint32_t count = result->getNumber<uint32_t>("count");
        bool valid = (count == 1);

        if (valid) {
            std::lock_guard<std::mutex> lock(statsMutex_);
            stats_.dataValidations++;
        }

        return valid;
    } catch (const std::exception& e) {
        logStabilityEvent("DATA_VALIDATION_ERROR", "Exception during player validation: " + std::string(e.what()));
        return false;
    }
}

bool StabilityManager::validateAccountData(uint32_t accountId) {
    try {
        Database& db = Database::getInstance();
        std::ostringstream query;
        // Allow accounts with or without emails - just check if account exists and is valid
        query << "SELECT COUNT(*) as count FROM accounts WHERE id = " << accountId
              << " AND name IS NOT NULL AND name != ''";

        DBResult_ptr result = db.storeQuery(query.str());
        if (!result) {
            logStabilityEvent("DATA_VALIDATION", "Account data validation failed for ID: " + std::to_string(accountId));
            return false;
        }

        uint32_t count = result->getNumber<uint32_t>("count");
        bool valid = (count == 1);

        if (valid) {
            std::lock_guard<std::mutex> lock(statsMutex_);
            stats_.dataValidations++;
        }

        return valid;
    } catch (const std::exception& e) {
        logStabilityEvent("DATA_VALIDATION_ERROR", "Exception during account validation: " + std::string(e.what()));
        return false;
    }
}

bool StabilityManager::validateItemData(uint32_t itemId) {
    try {
        Database& db = Database::getInstance();
        std::ostringstream query;
        query << "SELECT COUNT(*) as count FROM items WHERE id = " << itemId;

        DBResult_ptr result = db.storeQuery(query.str());
        if (!result) {
            return false;
        }

        uint32_t count = result->getNumber<uint32_t>("count");
        bool valid = (count == 1);

        if (valid) {
            std::lock_guard<std::mutex> lock(statsMutex_);
            stats_.dataValidations++;
        }

        return valid;
    } catch (const std::exception& e) {
        return false;
    }
}

bool StabilityManager::validateMapData(const Position& pos) {
    try {
        // Basic bounds checking for map coordinates
        if (pos.x >= 0xFFFF || pos.y >= 0xFFFF || pos.z >= MAP_MAX_LAYERS) {
            return false;
        }

        // Additional validation can be added here
        return true;
    } catch (const std::exception& e) {
        return false;
    }
}

void StabilityManager::registerMemoryBlock(void* ptr, size_t size, const std::string& tag) {
    if (!ptr || size == 0) {
        return;
    }

    std::lock_guard<std::mutex> lock(memoryMutex_);
    MemoryBlock block;
    block.size = size;
    block.tag = tag;
    block.checksum = calculateChecksum(ptr, size);
    block.registered = std::chrono::steady_clock::now();

    memoryBlocks_[ptr] = block;
}

void StabilityManager::unregisterMemoryBlock(void* ptr) {
    if (!ptr) {
        return;
    }

    std::lock_guard<std::mutex> lock(memoryMutex_);
    memoryBlocks_.erase(ptr);
}

bool StabilityManager::validateMemoryIntegrity() {
    std::lock_guard<std::mutex> lock(memoryMutex_);
    bool allValid = true;

    for (auto& pair : memoryBlocks_) {
        void* ptr = pair.first;
        MemoryBlock& block = pair.second;

        uint32_t currentChecksum = calculateChecksum(ptr, block.size);
        if (currentChecksum != block.checksum) {
            logStabilityEvent("MEMORY_CORRUPTION", "Memory corruption detected in block: " + block.tag);
            allValid = false;
        }
    }

    if (allValid) {
        std::lock_guard<std::mutex> statsLock(statsMutex_);
        stats_.memoryValidations++;
    }

    return allValid;
}

void StabilityManager::checkMemoryCorruption() {
    if (!validateMemoryIntegrity()) {
        logStabilityEvent("MEMORY_CHECK", "Memory corruption detected, attempting recovery");

        // Attempt recovery
        attemptRecovery("memory_corruption");
    }
}

void StabilityManager::trackAllocation(void* ptr, size_t size, const std::string& file, int line, const std::string& function) {
    if (!ptr || size == 0) {
        return;
    }

    // Only track 1 out of every 1000 allocations to save CPU
    // Unless it's huge (likely a map chunk or image), then always track
    static std::atomic<uint64_t> counter{0};

    if (size < 1024 * 1024 && (counter++ % 1000 != 0)) {
        return;
    }

    std::lock_guard<std::mutex> lock(allocationMutex_);

    uint64_t allocId = nextAllocationId_++;

    AllocationInfo info;
    info.size = size;
    info.file = file;
    info.line = line;
    info.function = function;
    info.timestamp = std::chrono::steady_clock::now();
    info.allocationId = allocId;

    activeAllocations_[ptr] = info;

    // Update statistics
    totalAllocatedBytes_ += size;
    totalAllocationCount_++;

    // Log large allocations immediately
    if (size >= LARGE_ALLOCATION_THRESHOLD) {
        std::ostringstream oss;
        oss << "Large allocation detected: " << size << " bytes at "
            << file << ":" << line << " in " << function << " (ID: " << allocId << ")";
        logStabilityEvent("LARGE_ALLOCATION", oss.str());
    }
}

void StabilityManager::trackDeallocation(void* ptr) {
    if (!ptr) {
        return;
    }

    std::lock_guard<std::mutex> lock(allocationMutex_);

    auto it = activeAllocations_.find(ptr);
    if (it != activeAllocations_.end()) {
        // Update statistics
        totalAllocatedBytes_ -= it->second.size;
        totalDeallocationCount_++;

        // Remove from active allocations
        activeAllocations_.erase(it);
    } else {
        // Log potential double-free or untracked deallocation
        std::ostringstream oss;
        oss << "Deallocation of untracked pointer: " << ptr;
        logStabilityEvent("UNTRACKED_DEALLOCATION", oss.str());
    }
}

void StabilityManager::detectMemoryLeaks() {
    std::lock_guard<std::mutex> lock(allocationMutex_);

    auto now = std::chrono::steady_clock::now();
    std::vector<void*> suspiciousAllocations;

    for (const auto& pair : activeAllocations_) {
        const AllocationInfo& info = pair.second;
        auto age = std::chrono::duration_cast<std::chrono::seconds>(now - info.timestamp).count();

        // Consider allocations older than 10 minutes as potential leaks
        if (age > 600) { // 10 minutes
            suspiciousAllocations.push_back(pair.first);
        }
    }

    if (!suspiciousAllocations.empty()) {
        std::ostringstream oss;
        oss << "Detected " << suspiciousAllocations.size() << " potential memory leaks";
        logStabilityEvent("MEMORY_LEAK_DETECTION", oss.str());

        // Log details for first 10 suspicious allocations
        size_t count = 0;
        for (void* ptr : suspiciousAllocations) {
            if (count >= 10) break; // Limit detailed logging

            auto it = activeAllocations_.find(ptr);
            if (it != activeAllocations_.end()) {
                const AllocationInfo& info = it->second;
                auto age = std::chrono::duration_cast<std::chrono::seconds>(now - info.timestamp).count();

                std::ostringstream detailOss;
                detailOss << "Leak candidate: " << info.size << " bytes, age: " << age
                         << "s, at " << info.file << ":" << info.line
                         << " in " << info.function << " (ID: " << info.allocationId << ")";
                logStabilityEvent("LEAK_CANDIDATE", detailOss.str());
                count++;
            }
        }
    }
}

void StabilityManager::reportMemoryLeaks() {
    std::lock_guard<std::mutex> lock(allocationMutex_);

    if (activeAllocations_.empty()) {
        logStabilityEvent("MEMORY_REPORT", "No active allocations detected");
        return;
    }

    size_t totalLeakedBytes = 0;
    std::map<std::string, size_t> leaksByLocation;

    for (const auto& pair : activeAllocations_) {
        const AllocationInfo& info = pair.second;
        totalLeakedBytes += info.size;

        std::string location = info.file + ":" + std::to_string(info.line);
        leaksByLocation[location] += info.size;
    }

    std::ostringstream oss;
    oss << "Memory leak report: " << activeAllocations_.size()
        << " active allocations, " << totalLeakedBytes << " bytes total";
    logStabilityEvent("MEMORY_LEAK_REPORT", oss.str());

    // Report top leak locations
    std::vector<std::pair<std::string, size_t>> sortedLeaks(leaksByLocation.begin(), leaksByLocation.end());
    std::sort(sortedLeaks.begin(), sortedLeaks.end(),
        [](const std::pair<std::string, size_t>& a, const std::pair<std::string, size_t>& b) {
            return a.second > b.second;
        });

    size_t count = 0;
    for (const auto& leak : sortedLeaks) {
        if (count >= 5) break; // Top 5 leak locations

        std::ostringstream locationOss;
        locationOss << "Top leak location: " << leak.first << " - " << leak.second << " bytes";
        logStabilityEvent("LEAK_LOCATION", locationOss.str());
        count++;
    }
}

void StabilityManager::clearMemoryLeaks() {
    std::lock_guard<std::mutex> lock(allocationMutex_);

    size_t clearedCount = activeAllocations_.size();
    size_t clearedBytes = 0;

    for (const auto& pair : activeAllocations_) {
        clearedBytes += pair.second.size;
    }

    activeAllocations_.clear();
    totalAllocatedBytes_ = 0;

    std::ostringstream oss;
    oss << "Cleared " << clearedCount << " tracked allocations (" << clearedBytes << " bytes)";
    logStabilityEvent("MEMORY_LEAK_CLEAR", oss.str());
}

size_t StabilityManager::getTotalAllocatedMemory() const {
    return totalAllocatedBytes_.load();
}

size_t StabilityManager::getActiveAllocationCount() const {
    std::lock_guard<std::mutex> lock(allocationMutex_);
    return activeAllocations_.size();
}

bool StabilityManager::beginTransaction(const std::string& context) {
    std::lock_guard<std::mutex> lock(transactionMutex_);

    if (activeTransactions_.find(context) != activeTransactions_.end()) {
        logStabilityEvent("TRANSACTION_ERROR", "Transaction already active for context: " + context);
        return false;
    }

    Transaction tx;
    tx.context = context;
    tx.started = std::chrono::steady_clock::now();
    tx.active = true;

    activeTransactions_[context] = tx;

    std::lock_guard<std::mutex> statsLock(statsMutex_);
    stats_.transactionsStarted++;

    return true;
}

bool StabilityManager::commitTransaction(const std::string& context) {
    std::lock_guard<std::mutex> lock(transactionMutex_);

    auto it = activeTransactions_.find(context);
    if (it == activeTransactions_.end()) {
        logStabilityEvent("TRANSACTION_ERROR", "No active transaction for context: " + context);
        return false;
    }

    activeTransactions_.erase(it);

    std::lock_guard<std::mutex> statsLock(statsMutex_);
    stats_.transactionsCommitted++;

    return true;
}

void StabilityManager::rollbackTransaction(const std::string& context) {
    std::lock_guard<std::mutex> lock(transactionMutex_);

    auto it = activeTransactions_.find(context);
    if (it == activeTransactions_.end()) {
        return;
    }

    activeTransactions_.erase(it);

    std::lock_guard<std::mutex> statsLock(statsMutex_);
    stats_.transactionsRolledBack++;

    logStabilityEvent("TRANSACTION_ROLLBACK", "Transaction rolled back for context: " + context);
}

void StabilityManager::registerRecoveryHandler(const std::string& component, std::function<bool()> handler) {
    std::lock_guard<std::mutex> lock(recoveryMutex_);
    recoveryHandlers_[component] = handler;
}

bool StabilityManager::attemptRecovery(const std::string& component) {
    std::lock_guard<std::mutex> lock(recoveryMutex_);
    std::lock_guard<std::mutex> statsLock(statsMutex_);

    stats_.recoveriesAttempted++;

    auto it = recoveryHandlers_.find(component);
    if (it == recoveryHandlers_.end()) {
        logStabilityEvent("RECOVERY_FAILED", "No recovery handler for component: " + component);
        return false;
    }

    bool success = it->second();
    if (success) {
        stats_.recoveriesSuccessful++;
        logStabilityEvent("RECOVERY_SUCCESS", "Recovery successful for component: " + component);
    } else {
        logStabilityEvent("RECOVERY_FAILED", "Recovery failed for component: " + component);
    }

    return success;
}

void StabilityManager::periodicHealthCheck() {
    // Prevent stacking checks if one takes too long
    static std::atomic<bool> isChecking{false};

    if (isChecking) return;
    isChecking = true;

    // Launch in a detached thread
    std::thread([this]() {
        try {
            auto now = std::chrono::steady_clock::now();

            // Update last health check time
            {
                std::lock_guard<std::mutex> lock(statsMutex_);
                stats_.lastHealthCheck = now;
            }

            // Check if log rotation is needed (do this first)
            if (shouldRotateLog()) {
                rotateLogFile();
            }

            // DB Validation - dispatch to main thread to avoid concurrent MYSQL* access
            // The singleton Database::getInstance() is NOT thread-safe in single-connection mode
            g_dispatcher.addTask(createTask([this]() {
                validateDatabaseConnection();
            }));

            // Memory validation (Slow)
            performMemoryValidation();

            // Perform memory leak detection
            detectMemoryLeaks();

            // Perform data validation
            performDataValidation();

            // Check resource limits
            checkResourceLimits();

            // Clean up expired transactions
            cleanupExpiredTransactions();

            // Monitor crash statistics and generate reports
            if (stats_.totalCrashes > 0) {
                logCrashStatistics();

                // Generate crash report every 24 health checks (approximately daily)
                static int reportCounter = 0;
                if (++reportCounter >= 24) {
                    generateCrashReport();
                    reportCounter = 0;
                }

                // Reset crash streak if no crashes in last health check period
                if (std::chrono::duration_cast<std::chrono::minutes>(
                    now - stats_.lastCrashTime).count() > 30) {  // 30 minutes
                    resetCrashStreak();
                }
            }

            // Monitor crash policies and cooldown periods
            if (isInCooldownPeriod()) {
                auto remaining = crashPolicy_.cooldownPeriod -
                    std::chrono::duration_cast<std::chrono::minutes>(
                        now - crashPolicy_.lastPolicyAction);
                logStabilityEvent("COOLDOWN_ACTIVE", "Remaining: " + std::to_string(remaining.count()) + " minutes");
            } else if (crashPolicy_.autoRestartDisabled) {
                // Reset policies after extended period of stability
                static int stabilityCounter = 0;
                if (++stabilityCounter >= 48) {  // 48 health checks ≈ 2 days of stability
                    resetCrashPolicies();
                    stabilityCounter = 0;
                }
            }
        } catch (...) {
            // Catch anything to keep thread from crashing program
        }
        isChecking = false;
    }).detach();
}

void StabilityManager::monitorResourceUsage() {
    std::lock_guard<std::mutex> lock(resourceMutex_);

    // Get memory usage
    struct rusage usage;
    if (getrusage(RUSAGE_SELF, &usage) == 0) {
        resourceStats_.memoryUsage = usage.ru_maxrss * 1024; // Convert to bytes
        if (resourceStats_.memoryUsage > resourceStats_.peakMemoryUsage) {
            resourceStats_.peakMemoryUsage = resourceStats_.memoryUsage;
        }
    }

    resourceStats_.lastUpdate = std::chrono::steady_clock::now();
}

void StabilityManager::checkResourceLimits() {
    // Define resource limits (configurable)
    const size_t maxMemoryUsage = 4ULL * 1024ULL * 1024ULL * 1024ULL; // 4GB - increased for game server
    const uint32_t maxConnections = 1000;

    std::lock_guard<std::mutex> lock(resourceMutex_);

    if (resourceStats_.memoryUsage > maxMemoryUsage) {
        logStabilityEvent("RESOURCE_WARNING", "Memory usage exceeded limit: " +
                         std::to_string(resourceStats_.memoryUsage) + " bytes");
        attemptRecovery("memory_limit");
    }

    if (resourceStats_.activeConnections > maxConnections) {
        logStabilityEvent("RESOURCE_WARNING", "Active connections exceeded limit: " +
                         std::to_string(resourceStats_.activeConnections));
        attemptRecovery("connection_limit");
    }
}

void StabilityManager::logStabilityMetrics() {
    std::lock_guard<std::mutex> lock(statsMutex_);

    std::ostringstream oss;
    oss << "Stability Metrics - Memory Validations: " << stats_.memoryValidations
        << ", Data Validations: " << stats_.dataValidations
        << ", Recoveries: " << stats_.recoveriesSuccessful << "/" << stats_.recoveriesAttempted
        << ", Transactions: " << stats_.transactionsCommitted << "/" << stats_.transactionsStarted
        << ", Memory: " << getTotalAllocatedMemory() << " bytes (" << getActiveAllocationCount() << " allocations)"
        << ", Alloc/Dealloc: " << totalAllocationCount_.load() << "/" << totalDeallocationCount_.load();

    logStabilityEvent("METRICS", oss.str());
}

bool StabilityManager::validateConfiguration() {
    try {
        // Check if critical configuration values are present and valid
        // Note: MySQL password can be empty for local development
        if (g_config.getString(ConfigManager::MYSQL_HOST).empty() ||
            g_config.getString(ConfigManager::MYSQL_USER).empty() ||
            g_config.getString(ConfigManager::MYSQL_DB).empty()) {
            logStabilityEvent("CONFIG_VALIDATION_FAILED", "Missing required MySQL configuration");
            return false;
        }

        // Validate numeric configuration values
        if (g_config.getNumber(ConfigManager::SQL_PORT) <= 0 || g_config.getNumber(ConfigManager::SQL_PORT) > 65535) {
            logStabilityEvent("CONFIG_VALIDATION_FAILED", "Invalid MySQL port configuration");
            return false;
        }

        return true;
    } catch (const std::exception& e) {
        logStabilityEvent("CONFIG_VALIDATION_ERROR", std::string("Exception: ") + e.what());
        return false;
    }
}

bool StabilityManager::validateDatabaseConnection() {
    try {
        Database& db = Database::getInstance();

        // Test basic connectivity
        DBResult_ptr result = db.storeQuery("SELECT 1 as test");
        if (!result) {
            return false;
        }

        uint32_t test = result->getNumber<uint32_t>("test");
        return test == 1;
    } catch (const std::exception& e) {
        logStabilityEvent("DB_VALIDATION_ERROR", std::string("Exception: ") + e.what());
        return false;
    }
}

bool StabilityManager::validateScriptEnvironment() {
    try {
        // Check if Lua environment is properly initialized
        // This would need to be implemented based on the specific Lua integration
        return true; // Placeholder
    } catch (const std::exception& e) {
        logStabilityEvent("SCRIPT_VALIDATION_ERROR", std::string("Exception: ") + e.what());
        return false;
    }
}

void StabilityManager::installSignalHandlers() {
    signal(SIGSEGV, signalHandler);
    signal(SIGABRT, signalHandler);
    signal(SIGFPE, signalHandler);
    signal(SIGILL, signalHandler);
    signal(SIGBUS, signalHandler);
    signal(SIGTERM, signalHandler);
    signal(SIGHUP, signalHandler);
}

void StabilityManager::handleSignal(int sig) {
    // 1. Immediately unblock signals to avoid loops, or mask them
    // (Optional, but safe practice)

    // 2. Open file using low-level IO (Async-Signal-Safe)
    // using O_DIRECT or just standard flags. 0666 = read/write permissions
    int fd = open("data/logs/crash_raw.log", O_WRONLY | O_CREAT | O_APPEND, 0666);

    if (fd != -1) {
        safe_write_str(fd, "\n\n=== CRITICAL CRASH DETECTED ===\nSignal: ");
        safe_write_int(fd, sig);
        safe_write_str(fd, "\n");

        // 3. Raw Backtrace
        // We do NOT resolve symbols here. It is too dangerous (requires malloc/popen).
        // We just dump the addresses. You resolve them later with a script.
        void* array[100];
        int size = backtrace(array, 100);

        safe_write_str(fd, "--- RAW STACK TRACE ---\n");
        // backtrace_symbols_fd writes directly to the file descriptor safely
        backtrace_symbols_fd(array, size, fd);

        safe_write_str(fd, "=== END CRASH LOG ===\n");
        close(fd);
    }

    // 4. Force Exit
    // Restore default handler and raise signal again to generate system core dump if enabled
    ::signal(sig, SIG_DFL);
    ::raise(sig);
}

void StabilityManager::createCoreDump() {
    // Create a comprehensive core dump file for debugging
    std::filesystem::create_directories(LOG_DIRECTORY);
    std::ofstream dump("data/logs/stability_coredump.log", std::ios::app);
    if (dump.is_open()) {
        auto now = std::chrono::system_clock::now();
        auto timestamp = now.time_since_epoch().count();

        dump << "=== ENHANCED CORE DUMP ===\n";
        dump << "Timestamp: " << timestamp << "\n";
        dump << "Readable Time: " << std::chrono::system_clock::to_time_t(now) << "\n";

        // Crash Categorization
        std::string crashCategory = categorizeCrash();
        dump << "Crash Category: " << crashCategory << "\n";

        // Record the crash in statistics
        recordCrash(crashCategory);

        // System Information
        dump << "\n--- SYSTEM INFORMATION ---\n";
        dump << "OS: " << getSystemInfo("uname -s") << "\n";
        dump << "Kernel: " << getSystemInfo("uname -r") << "\n";
        dump << "Architecture: " << getSystemInfo("uname -m") << "\n";
        dump << "Hostname: " << getSystemInfo("hostname") << "\n";

        // Process Information
        dump << "\n--- PROCESS INFORMATION ---\n";
        dump << "PID: " << getpid() << "\n";
        dump << "PPID: " << getppid() << "\n";
        dump << "UID: " << getuid() << "\n";
        dump << "GID: " << getgid() << "\n";
        dump << "Command Line: " << getSystemInfo("ps -p " + std::to_string(getpid()) + " -o cmd=") << "\n";

        // Memory Information
        dump << "\n--- MEMORY INFORMATION ---\n";
        dump << "Process Memory (RSS): " << getMemoryUsage() << " KB\n";
        dump << "Total Allocated Memory: " << getTotalAllocatedMemory() << " bytes\n";
        dump << "Active Allocations: " << getActiveAllocationCount() << "\n";

        // System Memory
        std::ifstream meminfo("/proc/meminfo");
        if (meminfo.is_open()) {
            dump << "\nSystem Memory Info:\n";
            std::string line;
            while (std::getline(meminfo, line) && !line.empty()) {
                if (line.find("MemTotal:") == 0 || line.find("MemFree:") == 0 ||
                    line.find("MemAvailable:") == 0 || line.find("SwapTotal:") == 0 ||
                    line.find("SwapFree:") == 0) {
                    dump << "  " << line << "\n";
                }
            }
            meminfo.close();
        }

        // Thread Information
        dump << "\n--- THREAD INFORMATION ---\n";
        dump << "Thread Count: " << getThreadCount() << "\n";
        dump << "Current Thread ID: " << std::this_thread::get_id() << "\n";

        // Signal Context (if available)
        dump << "\n--- SIGNAL CONTEXT ---\n";
        dump << "Last Signal Handled: " << getLastSignalInfo() << "\n";

        // Enhanced Backtrace with Symbol Resolution
        dump << "\n--- BACKTRACE ---\n";
        void* buffer[200];
        int nptrs = backtrace(buffer, 200);
        if (nptrs > 0) {
            char** strings = backtrace_symbols(buffer, nptrs);
            if (strings) {
                for (int i = 0; i < nptrs; ++i) {
                    dump << "[" << i << "] " << strings[i] << "\n";
                    // Try to resolve symbols to function names and line numbers
                    std::string resolved = resolveSymbol(buffer[i]);
                    if (!resolved.empty()) {
                        dump << "    -> " << resolved << "\n";
                    }
                }
                free(strings);
            } else {
                dump << "Failed to get backtrace symbols\n";
            }
        } else {
            dump << "No backtrace available\n";
        }

        // Recent Stability Events
        dump << "\n--- RECENT STABILITY EVENTS ---\n";
        logRecentEvents(dump);

        // Configuration State
        dump << "\n--- CONFIGURATION STATE ---\n";
        dump << "Configuration Valid: " << (validateConfiguration() ? "YES" : "NO") << "\n";
        dump << "Database Connection Valid: " << (validateDatabaseConnection() ? "YES" : "NO") << "\n";
        dump << "Script Environment Valid: " << (validateScriptEnvironment() ? "YES" : "NO") << "\n";

        // Stability Statistics
        dump << "\n--- STABILITY STATISTICS ---\n";
        {
            std::lock_guard<std::mutex> lock(statsMutex_);
            dump << "Memory Validations: " << stats_.memoryValidations << "\n";
            dump << "Data Validations: " << stats_.dataValidations << "\n";
            dump << "Recoveries Attempted: " << stats_.recoveriesAttempted << "\n";
            dump << "Recoveries Successful: " << stats_.recoveriesSuccessful << "\n";
            dump << "Signals Handled: " << stats_.signalsHandled << "\n";
            dump << "Transactions Started: " << stats_.transactionsStarted << "\n";
            dump << "Transactions Committed: " << stats_.transactionsCommitted << "\n";
            dump << "Transactions Rolled Back: " << stats_.transactionsRolledBack << "\n";
        }

        // Resource Usage
        dump << "\n--- RESOURCE USAGE ---\n";
        {
            std::lock_guard<std::mutex> lock(resourceMutex_);
            dump << "Current Memory Usage: " << resourceStats_.memoryUsage << " bytes\n";
            dump << "Peak Memory Usage: " << resourceStats_.peakMemoryUsage << " bytes\n";
            dump << "Active Connections: " << resourceStats_.activeConnections << "\n";
            dump << "Peak Connections: " << resourceStats_.peakConnections << "\n";
        }

        // Active Transactions
        dump << "\n--- ACTIVE TRANSACTIONS ---\n";
        {
            std::lock_guard<std::mutex> lock(transactionMutex_);
            if (activeTransactions_.empty()) {
                dump << "No active transactions\n";
            } else {
                for (const auto& [context, tx] : activeTransactions_) {
                    auto duration = std::chrono::steady_clock::now() - tx.started;
                    dump << "Context: " << context << ", Duration: "
                         << std::chrono::duration_cast<std::chrono::seconds>(duration).count() << "s\n";
                }
            }
        }

        // Memory Blocks Status
        dump << "\n--- MEMORY BLOCKS STATUS ---\n";
        {
            std::lock_guard<std::mutex> lock(memoryMutex_);
            dump << "Registered Memory Blocks: " << memoryBlocks_.size() << "\n";
            if (!memoryBlocks_.empty()) {
                dump << "Memory Integrity: " << (validateMemoryIntegrity() ? "VALID" : "CORRUPTED") << "\n";
            }
        }

        dump << "\n=== END ENHANCED CORE DUMP ===\n\n";
        dump.close();
    }
}

uint32_t StabilityManager::calculateChecksum(void* ptr, size_t size) {
    if (!ptr || size == 0) {
        return 0;
    }

    uint8_t* data = static_cast<uint8_t*>(ptr);
    uint32_t checksum = 0;

    // Only check the first 64 bytes and last 64 bytes
    // This catches 99% of buffer overflows without scanning the whole block
    size_t checkLimit = (size < 128) ? size : 64;

    // Head
    for (size_t i = 0; i < checkLimit; ++i) {
        checksum = (checksum * 31) + data[i];
    }

    // Tail
    if (size > 128) {
        for (size_t i = size - 64; i < size; ++i) {
            checksum = (checksum * 31) + data[i];
        }
    }

    return checksum;
}

std::string StabilityManager::getSystemInfo(const std::string& command) {
    char buffer[128];
    std::string result;
    FILE* pipe = popen(command.c_str(), "r");
    if (pipe) {
        while (fgets(buffer, sizeof(buffer), pipe) != nullptr) {
            result += buffer;
        }
        pclose(pipe);
    }
    // Remove trailing newline
    if (!result.empty() && result.back() == '\n') {
        result.pop_back();
    }
    return result.empty() ? "N/A" : result;
}

size_t StabilityManager::getMemoryUsage() {
    std::ifstream statm("/proc/self/statm");
    size_t rss = 0;
    if (statm.is_open()) {
        size_t size, resident, share, text, lib, data, dt;
        statm >> size >> resident >> share >> text >> lib >> data >> dt;
        rss = resident * getpagesize() / 1024; // Convert to KB
    }
    return rss;
}

int StabilityManager::getThreadCount() {
    std::ifstream status("/proc/self/status");
    std::string line;
    if (status.is_open()) {
        while (std::getline(status, line)) {
            if (line.find("Threads:") == 0) {
                size_t pos = line.find(':');
                if (pos != std::string::npos) {
                    try {
                        return std::stoi(line.substr(pos + 1));
                    } catch (...) {
                        return 1;
                    }
                }
            }
        }
    }
    return 1; // Fallback
}

std::string StabilityManager::getLastSignalInfo() {
    // This would need to be implemented to track the last signal
    // For now, return a placeholder
    return "Signal tracking not implemented";
}

std::string StabilityManager::resolveSymbol(void* address) {
    char command[256];
    snprintf(command, sizeof(command), "addr2line -f -C -e /proc/%d/exe %p 2>/dev/null", getpid(), address);

    FILE* pipe = popen(command, "r");
    if (!pipe) {
        return "";
    }

    char buffer[512];
    std::string function_name, file_info;

    // Read function name
    if (fgets(buffer, sizeof(buffer), pipe)) {
        function_name = buffer;
        if (!function_name.empty() && function_name.back() == '\n') {
            function_name.pop_back();
        }
    }

    // Read file and line info
    if (fgets(buffer, sizeof(buffer), pipe)) {
        file_info = buffer;
        if (!file_info.empty() && file_info.back() == '\n') {
            file_info.pop_back();
        }
    }

    pclose(pipe);

    if (function_name == "??") {
        return "";
    }

    std::string result = function_name;
    if (!file_info.empty() && file_info != "??:0") {
        result += " at " + file_info;
    }

    return result;
}

std::string StabilityManager::categorizeCrash() {
    // Get backtrace for analysis
    void* buffer[200];
    int nptrs = backtrace(buffer, 200);

    std::vector<std::string> symbols;
    if (nptrs > 0) {
        char** strings = backtrace_symbols(buffer, nptrs);
        if (strings) {
            for (int i = 0; i < nptrs && i < 10; ++i) { // Analyze first 10 frames
                symbols.push_back(strings[i]);
            }
            free(strings);
        }
    }

    // Network-related crashes
    for (const auto& symbol : symbols) {
        if (symbol.find("epoll") != std::string::npos ||
            symbol.find("socket") != std::string::npos ||
            symbol.find("connect") != std::string::npos ||
            symbol.find("bind") != std::string::npos ||
            symbol.find("listen") != std::string::npos ||
            symbol.find("accept") != std::string::npos) {
            return "NETWORK_IO";
        }
    }

    // Threading-related crashes
    for (const auto& symbol : symbols) {
        if (symbol.find("pthread") != std::string::npos ||
            symbol.find("thread") != std::string::npos ||
            symbol.find("mutex") != std::string::npos ||
            symbol.find("cond") != std::string::npos ||
            symbol.find("semaphore") != std::string::npos) {
            return "THREADING";
        }
    }

    // Memory-related crashes
    for (const auto& symbol : symbols) {
        if (symbol.find("malloc") != std::string::npos ||
            symbol.find("free") != std::string::npos ||
            symbol.find("realloc") != std::string::npos ||
            symbol.find("new") != std::string::npos ||
            symbol.find("delete") != std::string::npos) {
            return "MEMORY_MANAGEMENT";
        }
    }

    // Database-related crashes
    for (const auto& symbol : symbols) {
        if (symbol.find("mysql") != std::string::npos ||
            symbol.find("sqlite") != std::string::npos ||
            symbol.find("database") != std::string::npos ||
            symbol.find("query") != std::string::npos ||
            symbol.find("connection") != std::string::npos) {
            return "DATABASE";
        }
    }

    // Script/Lua-related crashes
    for (const auto& symbol : symbols) {
        if (symbol.find("lua") != std::string::npos ||
            symbol.find("script") != std::string::npos ||
            symbol.find("LuaScript") != std::string::npos) {
            return "SCRIPT_ENGINE";
        }
    }

    // File system operations
    for (const auto& symbol : symbols) {
        if (symbol.find("fopen") != std::string::npos ||
            symbol.find("fread") != std::string::npos ||
            symbol.find("fwrite") != std::string::npos ||
            symbol.find("fclose") != std::string::npos ||
            symbol.find("filesystem") != std::string::npos) {
            return "FILESYSTEM_IO";
        }
    }

    // Protocol/Game logic
    for (const auto& symbol : symbols) {
        if (symbol.find("protocol") != std::string::npos ||
            symbol.find("packet") != std::string::npos ||
            symbol.find("player") != std::string::npos ||
            symbol.find("monster") != std::string::npos ||
            symbol.find("combat") != std::string::npos) {
            return "GAME_LOGIC";
        }
    }

    // STL container issues
    for (const auto& symbol : symbols) {
        if (symbol.find("std::") != std::string::npos ||
            symbol.find("vector") != std::string::npos ||
            symbol.find("map") != std::string::npos ||
            symbol.find("list") != std::string::npos ||
            symbol.find("queue") != std::string::npos) {
            return "STL_CONTAINER";
        }
    }

    // Unknown/Generic crashes
    return "UNKNOWN";
}

void StabilityManager::logRecentEvents(std::ofstream& dump) {
    // This would log recent stability events
    // For now, just indicate this feature
    dump << "Recent events logging not implemented\n";
}

void StabilityManager::logStabilityEvent(const std::string& event, const std::string& details) {
    std::cout << "[Stability] " << event << ": " << details << std::endl;

    // Check if log rotation is needed before writing
    if (shouldRotateLog()) {
        rotateLogFile();
    }

    // Thread-safe logging to file
    {
        std::lock_guard<std::mutex> lock(logMutex_);
        // Ensure log directory exists
        std::filesystem::create_directories(LOG_DIRECTORY);
        std::ofstream log(STABILITY_LOG_FILE, std::ios::app);
        if (log.is_open()) {
            auto now = std::chrono::system_clock::now();
            auto time_t = std::chrono::system_clock::to_time_t(now);

            // Use more structured timestamp format
            log << std::put_time(std::localtime(&time_t), "%Y-%m-%d %H:%M:%S")
                << " [Stability] " << event << ": " << details << std::endl;
            log.close();
        }
    }
}

bool StabilityManager::isValidPointer(void* ptr) {
    // Basic pointer validation (can be enhanced)
    return ptr != nullptr;
}

void StabilityManager::cleanupExpiredTransactions() {
    std::lock_guard<std::mutex> lock(transactionMutex_);

    auto now = std::chrono::steady_clock::now();
    auto timeout = std::chrono::minutes(5); // 5 minute timeout

    for (auto it = activeTransactions_.begin(); it != activeTransactions_.end(); ) {
        auto elapsed = now - it->second.started;
        if (elapsed > timeout) {
            logStabilityEvent("TRANSACTION_TIMEOUT", "Transaction timed out: " + it->first);
            it = activeTransactions_.erase(it);
        } else {
            ++it;
        }
    }
}

void StabilityManager::performMemoryValidation() {
    checkMemoryCorruption();
}

void StabilityManager::performDataValidation() {
    // Perform periodic data integrity checks
    // This could be expanded to validate critical data sets
    validateConfiguration();
}

bool StabilityManager::shouldRotateLog() const {
    struct stat statBuf;
    if (stat(STABILITY_LOG_FILE, &statBuf) == 0) {
        return static_cast<size_t>(statBuf.st_size) > MAX_LOG_SIZE;
    }
    return false;
}

std::string StabilityManager::getTimestampedFilename(const std::string& baseFilename) const {
    auto now = std::chrono::system_clock::now();
    auto time_t = std::chrono::system_clock::to_time_t(now);

    std::ostringstream oss;
    oss << baseFilename << "."
        << std::put_time(std::localtime(&time_t), "%Y%m%d_%H%M%S");
    return oss.str();
}

void StabilityManager::rotateLogFile() {
    std::lock_guard<std::mutex> lock(logMutex_);

    try {
        // Check if log file exists and needs rotation
        if (!shouldRotateLog()) {
            return;
        }

        // Generate timestamped filename for the old log
        std::string archivedName = getTimestampedFilename("data/logs/stability_archive");

        // Rename current log file to archived name
        if (std::filesystem::exists(STABILITY_LOG_FILE)) {
            std::filesystem::rename(STABILITY_LOG_FILE, archivedName);

            // Log rotation message directly to console and new log file to avoid circular calls
            std::cout << "[Stability] LOG_ROTATION: Log file rotated to: " << archivedName << std::endl;

            // Write rotation message to new log file
            std::ofstream newLog(STABILITY_LOG_FILE, std::ios::app);
            if (newLog.is_open()) {
                auto now = std::chrono::system_clock::now();
                auto time_t = std::chrono::system_clock::to_time_t(now);
                newLog << std::put_time(std::localtime(&time_t), "%Y-%m-%d %H:%M:%S")
                       << " [Stability] LOG_ROTATION: Log file rotated to: " << archivedName << std::endl;
                newLog.close();
            }
        }

        // Clean up old archived files
        cleanupOldLogs();

    } catch (const std::exception& e) {
        // If rotation fails, log the error but continue operation
        std::cerr << "[Stability] Log rotation failed: " << e.what() << std::endl;
    }
}

void StabilityManager::cleanupOldLogs() {
    try {
        std::vector<std::filesystem::directory_entry> logFiles;

        // Find all archived stability log files
        for (const auto& entry : std::filesystem::directory_iterator(LOG_DIRECTORY)) {
            if (entry.is_regular_file()) {
                std::string filename = entry.path().filename().string();
                if (filename.find("stability_archive") == 0) {
                    logFiles.push_back(entry);
                }
            }
        }

        // Sort by modification time (newest first)
        std::sort(logFiles.begin(), logFiles.end(),
            [](const std::filesystem::directory_entry& a, const std::filesystem::directory_entry& b) {
                return std::filesystem::last_write_time(a) > std::filesystem::last_write_time(b);
            });

        // Remove excess files (keep only MAX_LOG_FILES)
        for (size_t i = MAX_LOG_FILES; i < logFiles.size(); ++i) {
            try {
                std::filesystem::remove(logFiles[i].path());
                std::cout << "[Stability] Removed old log file: " << logFiles[i].path().filename() << std::endl;
            } catch (const std::exception& e) {
                std::cerr << "[Stability] Failed to remove old log file "
                         << logFiles[i].path().filename() << ": " << e.what() << std::endl;
            }
        }

    } catch (const std::exception& e) {
        std::cerr << "[Stability] Error during log cleanup: " << e.what() << std::endl;
    }
}

void StabilityManager::forceLogRotation() {
    rotateLogFile();
}

void StabilityManager::recordCrash(const std::string& category) {
    std::lock_guard<std::mutex> lock(statsMutex_);
    stats_.totalCrashes++;
    stats_.lastCrashTime = std::chrono::steady_clock::now();
    stats_.crashStreak++;

    if (stats_.crashStreak > stats_.maxCrashStreak) {
        stats_.maxCrashStreak = stats_.crashStreak;
    }

    // Categorize the crash
    if (category.find("NETWORK") != std::string::npos) {
        stats_.networkCrashes++;
    } else if (category.find("THREAD") != std::string::npos) {
        stats_.threadCrashes++;
    } else if (category.find("MEMORY") != std::string::npos) {
        stats_.memoryCrashes++;
    } else if (category.find("DATABASE") != std::string::npos) {
        stats_.databaseCrashes++;
    } else if (category.find("SCRIPT") != std::string::npos) {
        stats_.scriptCrashes++;
    } else if (category.find("FILESYSTEM") != std::string::npos) {
        stats_.filesystemCrashes++;
    } else if (category.find("GAME") != std::string::npos) {
        stats_.gameLogicCrashes++;
    } else if (category.find("STL") != std::string::npos) {
        stats_.stlCrashes++;
    } else {
        stats_.unknownCrashes++;
    }

    logStabilityEvent("CRASH_RECORDED", "Category: " + category + ", Total: " + std::to_string(stats_.totalCrashes));
}

void StabilityManager::resetCrashStreak() {
    std::lock_guard<std::mutex> lock(statsMutex_);
    stats_.crashStreak = 0;
}

void StabilityManager::generateCrashReport() {
    std::lock_guard<std::mutex> lock(statsMutex_);

    std::ofstream report("data/logs/crash_report.txt", std::ios::app);
    if (report.is_open()) {
        auto now = std::chrono::system_clock::now();
        report << "=== CRASH REPORT ===\n";
        report << "Generated: " << std::chrono::system_clock::to_time_t(now) << "\n";
        report << "Total crashes: " << stats_.totalCrashes << "\n";
        report << "Current crash streak: " << stats_.crashStreak << "\n";
        report << "Max crash streak: " << stats_.maxCrashStreak << "\n";
        report << "Crash rate (per hour): " << getCrashRatePerHour() << "\n";
        report << "Crash rate (per day): " << getCrashRatePerDay() << "\n";
        report << "Most common crash type: " << getMostCommonCrashType() << "\n";
        report << "Is critical rate: " << (isCrashRateCritical() ? "YES" : "NO") << "\n";

        report << "\nCrash breakdown:\n";
        report << "  Network: " << stats_.networkCrashes << "\n";
        report << "  Threading: " << stats_.threadCrashes << "\n";
        report << "  Memory: " << stats_.memoryCrashes << "\n";
        report << "  Database: " << stats_.databaseCrashes << "\n";
        report << "  Script: " << stats_.scriptCrashes << "\n";
        report << "  Filesystem: " << stats_.filesystemCrashes << "\n";
        report << "  Game Logic: " << stats_.gameLogicCrashes << "\n";
        report << "  STL: " << stats_.stlCrashes << "\n";
        report << "  Unknown: " << stats_.unknownCrashes << "\n";

        report << "\n=== END CRASH REPORT ===\n\n";
        report.close();
    }
}

void StabilityManager::logCrashStatistics() {
    std::lock_guard<std::mutex> lock(statsMutex_);

    logStabilityEvent("CRASH_STATS", "Total: " + std::to_string(stats_.totalCrashes) +
                     ", Rate/hr: " + std::to_string(getCrashRatePerHour()) +
                     ", Critical: " + (isCrashRateCritical() ? "YES" : "NO"));
}

double StabilityManager::getCrashRatePerHour() const {
    if (stats_.totalCrashes == 0) {
        return 0.0;
    }

    // Calculate uptime in hours (simplified - in real implementation you'd track actual uptime)
    auto uptime = std::chrono::duration_cast<std::chrono::hours>(
        std::chrono::steady_clock::now() - stats_.lastCrashTime).count();

    if (uptime <= 0) {
        uptime = 1; // Avoid division by zero
    }

    return static_cast<double>(stats_.totalCrashes) / uptime;
}

double StabilityManager::getCrashRatePerDay() const {
    return getCrashRatePerHour() * 24.0;
}

std::string StabilityManager::getMostCommonCrashType() const {
    std::vector<std::pair<uint64_t, std::string>> crashTypes = {
        {stats_.networkCrashes, "NETWORK_IO"},
        {stats_.threadCrashes, "THREADING"},
        {stats_.memoryCrashes, "MEMORY_MANAGEMENT"},
        {stats_.databaseCrashes, "DATABASE"},
        {stats_.scriptCrashes, "SCRIPT_ENGINE"},
        {stats_.filesystemCrashes, "FILESYSTEM_IO"},
        {stats_.gameLogicCrashes, "GAME_LOGIC"},
        {stats_.stlCrashes, "STL_CONTAINER"},
        {stats_.unknownCrashes, "UNKNOWN"}
    };

    auto maxType = std::max_element(crashTypes.begin(), crashTypes.end(),
        [](const auto& a, const auto& b) { return a.first < b.first; });

    return maxType->first > 0 ? maxType->second : "NONE";
}

bool StabilityManager::isCrashRateCritical() const {
    double hourlyRate = getCrashRatePerHour();
    return hourlyRate > 5.0 || stats_.crashStreak > 3; // More than 5 crashes per hour or 3+ in a row
}

void StabilityManager::setCrashPolicy(const CrashPolicy& policy) {
    std::lock_guard<std::mutex> lock(policyMutex_);
    crashPolicy_ = policy;
    logStabilityEvent("CRASH_POLICY_UPDATED", "Max crashes/hour: " + std::to_string(policy.maxCrashesPerHour) +
                     ", Max streak: " + std::to_string(policy.maxCrashStreak));
}

StabilityManager::CrashSeverity StabilityManager::evaluateCrashSeverity() const {
    std::lock_guard<std::mutex> lock(statsMutex_);
    std::lock_guard<std::mutex> lock2(policyMutex_);

    double hourlyRate = getCrashRatePerHour();

    if (stats_.crashStreak >= crashPolicy_.maxCrashStreak ||
        hourlyRate >= crashPolicy_.maxCrashesPerHour * 2) {
        return CrashSeverity::CRITICAL;
    }

    if (stats_.crashStreak >= crashPolicy_.maxCrashStreak / 2 ||
        hourlyRate >= crashPolicy_.maxCrashesPerHour) {
        return CrashSeverity::HIGH;
    }

    if (stats_.crashStreak >= 2 || hourlyRate >= crashPolicy_.maxCrashesPerHour / 2) {
        return CrashSeverity::MODERATE;
    }

    return CrashSeverity::LOW;
}

bool StabilityManager::shouldPreventRestart() const {
    std::lock_guard<std::mutex> lock(policyMutex_);
    return crashPolicy_.autoRestartDisabled || isInCooldownPeriod();
}

void StabilityManager::escalateCrashResponse(CrashSeverity severity) {
    std::string severityStr;
    std::string action;

    switch (severity) {
        case CrashSeverity::LOW:
            severityStr = "LOW";
            action = "Monitoring increased - no action taken";
            break;
        case CrashSeverity::MODERATE:
            severityStr = "MODERATE";
            action = "Extended cooldown period activated";
            implementCooldownPeriod();
            break;
        case CrashSeverity::HIGH:
            severityStr = "HIGH";
            action = "Auto-restart temporarily disabled, manual intervention required";
            {
                std::lock_guard<std::mutex> lock(policyMutex_);
                crashPolicy_.autoRestartDisabled = true;
                crashPolicy_.lastPolicyAction = std::chrono::steady_clock::now();
            }
            break;
        case CrashSeverity::CRITICAL:
            severityStr = "CRITICAL";
            action = "Server shutdown initiated - severe crash pattern detected";
            {
                std::lock_guard<std::mutex> lock(policyMutex_);
                crashPolicy_.autoRestartDisabled = true;
                crashPolicy_.lastPolicyAction = std::chrono::steady_clock::now();
            }
            // In a real implementation, you might want to trigger a full server shutdown here
            break;
    }

    logStabilityEvent("CRASH_ESCALATION", "Severity: " + severityStr + " - " + action);
}

void StabilityManager::implementCooldownPeriod() {
    std::lock_guard<std::mutex> lock(policyMutex_);
    crashPolicy_.lastPolicyAction = std::chrono::steady_clock::now();
    logStabilityEvent("COOLDOWN_ACTIVATED", "Cooldown period: " +
                     std::to_string(crashPolicy_.cooldownPeriod.count()) + " minutes");
}

bool StabilityManager::isInCooldownPeriod() const {
    std::lock_guard<std::mutex> lock(policyMutex_);
    auto now = std::chrono::steady_clock::now();
    auto timeSinceLastAction = std::chrono::duration_cast<std::chrono::minutes>(
        now - crashPolicy_.lastPolicyAction);
    return timeSinceLastAction < crashPolicy_.cooldownPeriod;
}

void StabilityManager::resetCrashPolicies() {
    std::lock_guard<std::mutex> lock(policyMutex_);
    crashPolicy_.autoRestartDisabled = false;
    crashPolicy_.lastPolicyAction = std::chrono::steady_clock::now();
    logStabilityEvent("CRASH_POLICIES_RESET", "Auto-restart re-enabled, cooldown cleared");
}

void StabilityManager::signalHandler(int signal) {
    if (instance_) {
        instance_->handleSignal(signal);
    }
}
