// Copyright 2022 The Forgotten Server Authors. All rights reserved.
// Use of this source code is governed by the GPL-2.0 License that can be found in the LICENSE file.

#ifndef FS_CONNECTIONPOOL_H_8B4C9F2E5A1D4C8B9F7E6A3D8B1C9F2E
#define FS_CONNECTIONPOOL_H_8B4C9F2E5A1D4C8B9F7E6A3D8B1C9F2E

#include <mysql/mysql.h>
#include <memory>
#include <queue>
#include <mutex>
#include <condition_variable>
#include <atomic>
#include <chrono>
#include <thread>

class ConnectionPool
{
public:
    struct Connection
    {
        MYSQL* handle = nullptr;
        std::chrono::steady_clock::time_point lastUsed;
        bool inUse = false;

        Connection();
        ~Connection();

        // non-copyable
        Connection(const Connection&) = delete;
        Connection& operator=(const Connection&) = delete;

        bool connect(const std::string& host, const std::string& user, const std::string& pass,
                    const std::string& db, unsigned int port, const std::string& socket);
        void disconnect();
        bool isConnected() const;
        bool ping();
    };

    // Pool statistics for monitoring
    struct PoolStats {
        std::atomic<uint64_t> totalAcquires{0};
        std::atomic<uint64_t> totalReleases{0};
        std::atomic<uint64_t> failedAcquires{0};
        std::atomic<uint64_t> reconnections{0};
        std::atomic<uint64_t> validationFailures{0};
    };

    explicit ConnectionPool(size_t poolSize = 10, size_t maxPoolSize = 50,
                           std::chrono::seconds connectionTimeout = std::chrono::seconds(300),
                           std::chrono::seconds acquireTimeout = std::chrono::seconds(30),
                           size_t minPoolSize = 1);
    ~ConnectionPool();

    // non-copyable
    ConnectionPool(const ConnectionPool&) = delete;
    ConnectionPool& operator=(const ConnectionPool&) = delete;

    /**
     * Initialize the connection pool with database connection parameters
     *
     * @param host MySQL host
     * @param user MySQL username
     * @param pass MySQL password
     * @param db MySQL database name
     * @param port MySQL port
     * @param socket MySQL socket path (optional)
     * @return true on success, false on error
     */
    bool initialize(const std::string& host, const std::string& user, const std::string& pass,
                   const std::string& db, unsigned int port, const std::string& socket = "");

    /**
     * Acquire a connection from the pool
     *
     * @return shared pointer to connection, nullptr if timeout or error
     */
    std::shared_ptr<Connection> acquire();

    /**
     * Release a connection back to the pool
     *
     * @param connection the connection to release
     */
    void release(std::shared_ptr<Connection> connection);

    /**
     * Get current pool statistics (thread-safe snapshot)
     */
    size_t getPoolSize() const { return poolSize_; }
    size_t getMinPoolSize() const { return minPoolSize_; }
    size_t getActiveConnections() const;   // Thread-safe
    size_t getAvailableConnections() const;  // Thread-safe
    size_t getTotalConnections() const;    // Thread-safe

    /**
     * Get a consistent snapshot of all pool metrics
     */
    struct PoolSnapshot {
        size_t poolSize;
        size_t minPoolSize;
        size_t maxPoolSize;
        size_t totalConnections;
        size_t activeConnections;
        size_t availableConnections;
        uint64_t totalAcquires;
        uint64_t totalReleases;
        uint64_t failedAcquires;
        uint64_t reconnections;
        uint64_t validationFailures;
    };
    PoolSnapshot getSnapshot() const;

    /**
     * Get pool statistics reference for monitoring
     */
    const PoolStats& getStats() const { return stats_; }

    /**
     * Cleanup idle connections
     */
    void cleanup();

    /**
     * Shutdown the connection pool
     */
    void shutdown();

    /**
     * Check if an error code is recoverable (connection lost, etc.)
     */
    static bool isRecoverableError(unsigned int errorCode);

private:
    std::shared_ptr<Connection> createConnection();
    bool validateConnection(std::shared_ptr<Connection> connection, bool thorough = false);
    void cleanupTask();

    // Configuration
    size_t poolSize_;
    size_t maxPoolSize_;
    size_t minPoolSize_;
    std::chrono::seconds connectionTimeout_;
    std::chrono::seconds acquireTimeout_;
    std::chrono::seconds idleValidationThreshold_{60};  // Validate connections idle longer than this

    // Connection parameters
    std::string host_;
    std::string user_;
    std::string pass_;
    std::string db_;
    unsigned int port_;
    std::string socket_;

    // Pool state
    std::queue<std::shared_ptr<Connection>> availableConnections_;
    std::atomic<size_t> totalConnections_;  // Total connections created
    std::atomic<size_t> inUseConnections_;  // Connections currently borrowed
    std::atomic<bool> initialized_;
    std::atomic<bool> shutdown_;

    // Threading
    mutable std::mutex poolMutex_;
    std::condition_variable poolCondition_;
    std::thread cleanupThread_;

    // Statistics
    PoolStats stats_;
};

#endif
