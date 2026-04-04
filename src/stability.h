#ifndef FS_STABILITY_H
#define FS_STABILITY_H

#include <memory>
#include <mutex>
#include <atomic>
#include <unordered_map>
#include <vector>
#include <string>
#include <chrono>
#include <functional>
#include <map>

class Database;
class Player;
class Item;
struct Position;

// Memory leak detection macros - only enabled in debug builds or when explicitly enabled
#ifdef ENABLE_MEMORY_LEAK_DETECTION
#define STABILITY_TRACK_ALLOC(ptr, size) \
    StabilityManager::getInstance().trackAllocation(ptr, size, __FILE__, __LINE__, __FUNCTION__)
#define STABILITY_TRACK_DEALLOC(ptr) \
    StabilityManager::getInstance().trackDeallocation(ptr)
#else
#define STABILITY_TRACK_ALLOC(ptr, size) do {} while(0)
#define STABILITY_TRACK_DEALLOC(ptr) do {} while(0)
#endif

class StabilityManager {
public:
    static StabilityManager& getInstance();

    // Core stability features
    void initialize();
    void shutdown();

    // Data integrity validation
    bool validatePlayerData(uint32_t playerId);
    bool validateAccountData(uint32_t accountId);
    bool validateItemData(uint32_t itemId);
    bool validateMapData(const Position& pos);

    // Memory corruption detection
    void registerMemoryBlock(void* ptr, size_t size, const std::string& tag);
    void unregisterMemoryBlock(void* ptr);
    bool validateMemoryIntegrity();
    void checkMemoryCorruption();

    // Memory leak detection
    void trackAllocation(void* ptr, size_t size, const std::string& file, int line, const std::string& function);
    void trackDeallocation(void* ptr);
    void detectMemoryLeaks();
    void reportMemoryLeaks();
    void clearMemoryLeaks();
    size_t getTotalAllocatedMemory() const;
    size_t getActiveAllocationCount() const;

    // Transaction safety
    bool beginTransaction(const std::string& context);
    bool commitTransaction(const std::string& context);
    void rollbackTransaction(const std::string& context);

    // Automatic recovery
    void registerRecoveryHandler(const std::string& component, std::function<bool()> handler);
    bool attemptRecovery(const std::string& component);
    void periodicHealthCheck();

    // Resource monitoring
    void monitorResourceUsage();
    void checkResourceLimits();
    void logStabilityMetrics();

    // Configuration validation
    bool validateConfiguration();
    bool validateDatabaseConnection();
    bool validateScriptEnvironment();

    // Crash prevention
    void installSignalHandlers();
    void handleSignal(int signal);
    void createCoreDump();
    void recordCrash(const std::string& category);
    void resetCrashStreak();

    // Log management
    void forceLogRotation();

    // Crash statistics and reporting
    void generateCrashReport();
    void logCrashStatistics();
    double getCrashRatePerHour() const;
    double getCrashRatePerDay() const;
    std::string getMostCommonCrashType() const;
    bool isCrashRateCritical() const;

    // Crash rate limiting and escalation
    enum class CrashSeverity { LOW, MODERATE, HIGH, CRITICAL };
    struct CrashPolicy {
        uint32_t maxCrashesPerHour = 5;
        uint32_t maxCrashStreak = 3;
        std::chrono::minutes cooldownPeriod = std::chrono::minutes(10);
        bool autoRestartDisabled = false;
        std::chrono::steady_clock::time_point lastPolicyAction;
    };

    void setCrashPolicy(const CrashPolicy& policy);
    CrashSeverity evaluateCrashSeverity() const;
    bool shouldPreventRestart() const;
    void escalateCrashResponse(CrashSeverity severity);
    void implementCooldownPeriod();
    bool isInCooldownPeriod() const;
    void resetCrashPolicies();

    // Statistics and reporting
    struct StabilityStats {
        uint64_t memoryValidations = 0;
        uint64_t dataValidations = 0;
        uint64_t recoveriesAttempted = 0;
        uint64_t recoveriesSuccessful = 0;
        uint64_t transactionsStarted = 0;
        uint64_t transactionsCommitted = 0;
        uint64_t transactionsRolledBack = 0;
        uint64_t signalsHandled = 0;
        std::chrono::steady_clock::time_point lastHealthCheck;

        // Crash statistics
        uint64_t totalCrashes = 0;
        uint64_t networkCrashes = 0;
        uint64_t threadCrashes = 0;
        uint64_t memoryCrashes = 0;
        uint64_t databaseCrashes = 0;
        uint64_t scriptCrashes = 0;
        uint64_t filesystemCrashes = 0;
        uint64_t gameLogicCrashes = 0;
        uint64_t stlCrashes = 0;
        uint64_t unknownCrashes = 0;
        std::chrono::steady_clock::time_point lastCrashTime;
        uint64_t crashStreak = 0;
        uint64_t maxCrashStreak = 0;
    };

    const StabilityStats& getStats() const { return stats_; }

private:
    StabilityManager() = default;
    ~StabilityManager() = default;
    StabilityManager(const StabilityManager&) = delete;
    StabilityManager& operator=(const StabilityManager&) = delete;

    // Memory tracking
    struct MemoryBlock {
        size_t size;
        std::string tag;
        uint32_t checksum;
        std::chrono::steady_clock::time_point registered;
    };

    std::unordered_map<void*, MemoryBlock> memoryBlocks_;
    std::mutex memoryMutex_;

    // Memory leak detection
    struct AllocationInfo {
        size_t size;
        std::string file;
        int line;
        std::string function;
        std::chrono::steady_clock::time_point timestamp;
        uint64_t allocationId;
    };

    std::unordered_map<void*, AllocationInfo> activeAllocations_;
    std::atomic<uint64_t> nextAllocationId_;
    std::atomic<size_t> totalAllocatedBytes_;
    std::atomic<size_t> totalAllocationCount_;
    std::atomic<size_t> totalDeallocationCount_;
    mutable std::mutex allocationMutex_;

    // Transaction management
    struct Transaction {
        std::string context;
        std::chrono::steady_clock::time_point started;
        bool active = false;
    };

    std::unordered_map<std::string, Transaction> activeTransactions_;
    std::mutex transactionMutex_;

    // Recovery handlers
    std::unordered_map<std::string, std::function<bool()>> recoveryHandlers_;
    std::mutex recoveryMutex_;

    // Resource monitoring
    struct ResourceStats {
        size_t memoryUsage = 0;
        size_t peakMemoryUsage = 0;
        uint32_t activeConnections = 0;
        uint32_t peakConnections = 0;
        std::chrono::steady_clock::time_point lastUpdate;
    };

    ResourceStats resourceStats_;
    std::mutex resourceMutex_;

    // Statistics
    StabilityStats stats_;
    mutable std::mutex statsMutex_;

    // Crash policy and rate limiting
    CrashPolicy crashPolicy_;
    mutable std::mutex policyMutex_;

    // Log rotation settings
    static constexpr size_t MAX_LOG_SIZE = 10 * 1024 * 1024; // 10MB
    static constexpr size_t MAX_LOG_FILES = 5; // Keep 5 archived log files
    static constexpr const char* LOG_DIRECTORY = "data/logs/";
    static constexpr const char* STABILITY_LOG_FILE = "data/logs/stability.log";
    mutable std::mutex logMutex_;

    // Memory leak detection settings
    static constexpr size_t LEAK_DETECTION_THRESHOLD = 1000; // Report if allocation lives longer than 1000 checks
    static constexpr size_t LARGE_ALLOCATION_THRESHOLD = 1024 * 1024; // 1MB - log large allocations immediately

    // Helper methods
    uint32_t calculateChecksum(void* ptr, size_t size);
    void logStabilityEvent(const std::string& event, const std::string& details);
    bool isValidPointer(void* ptr);
    void cleanupExpiredTransactions();
    void performMemoryValidation();
    void performDataValidation();

    // Enhanced coredump helpers
    std::string getSystemInfo(const std::string& command);
    size_t getMemoryUsage();
    int getThreadCount();
    std::string getLastSignalInfo();
    void logRecentEvents(std::ofstream& dump);
    std::string resolveSymbol(void* address);
    std::string categorizeCrash();

    // Log rotation methods
    void rotateLogFile();
    bool shouldRotateLog() const;
    void cleanupOldLogs();
    std::string getTimestampedFilename(const std::string& baseFilename) const;

    // Signal handling
    static void signalHandler(int signal);
    static StabilityManager* instance_;
};

#endif // FS_STABILITY_H
