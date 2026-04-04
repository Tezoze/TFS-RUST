// Copyright 2022 The Forgotten Server Authors. All rights reserved.
// Use of this source code is governed by the GPL-2.0 License that can be found in the LICENSE file.

#include "otpch.h"

#include "connectionpool.h"
#include "configmanager.h"

#include <iostream>
#include <algorithm>
#include <mysql/errmsg.h>

extern ConfigManager g_config;

ConnectionPool::Connection::Connection() {}

ConnectionPool::Connection::~Connection()
{
    disconnect();
}

bool ConnectionPool::Connection::connect(const std::string& host, const std::string& user, const std::string& pass,
                                       const std::string& db, unsigned int port, const std::string& socket)
{
    disconnect();

    handle = mysql_init(nullptr);
    if (!handle) {
        std::cout << "[Error - ConnectionPool::Connection::connect] Failed to initialize MySQL connection handle" << std::endl;
        return false;
    }

    // Note: MYSQL_OPT_RECONNECT is deprecated and handled at application level
    // Connection validation and reconnection is managed by the connection pool

    // connect to database
    if (!mysql_real_connect(handle, host.c_str(), user.c_str(), pass.c_str(),
                           db.c_str(), port, socket.empty() ? nullptr : socket.c_str(), 0)) {
        std::cout << "[Error - ConnectionPool::Connection::connect] MySQL Error Message: " << mysql_error(handle) << std::endl;
        disconnect();
        return false;
    }

    lastUsed = std::chrono::steady_clock::now();
    inUse = false;
    return true;
}

void ConnectionPool::Connection::disconnect()
{
    if (handle) {
        mysql_close(handle);
        handle = nullptr;
    }
    inUse = false;
}

bool ConnectionPool::Connection::isConnected() const
{
    return handle != nullptr;
}

bool ConnectionPool::Connection::ping()
{
    if (!handle) {
        return false;
    }

    int result = mysql_ping(handle);
    if (result == 0) {
        lastUsed = std::chrono::steady_clock::now();
        return true;
    }

    std::cout << "[Warning - ConnectionPool::Connection::ping] Connection lost, error: " << mysql_error(handle) << std::endl;
    return false;
}

ConnectionPool::ConnectionPool(size_t poolSize, size_t maxPoolSize,
                             std::chrono::seconds connectionTimeout,
                             std::chrono::seconds acquireTimeout,
                             size_t minPoolSize)
    : poolSize_(poolSize),
      maxPoolSize_(maxPoolSize),
      minPoolSize_(std::max(size_t(1), std::min(minPoolSize, poolSize))),
      connectionTimeout_(connectionTimeout),
      acquireTimeout_(acquireTimeout),
      totalConnections_(0),
      inUseConnections_(0),
      initialized_(false),
      shutdown_(false)
{
}

ConnectionPool::~ConnectionPool()
{
    shutdown();
}

bool ConnectionPool::initialize(const std::string& host, const std::string& user, const std::string& pass,
                              const std::string& db, unsigned int port, const std::string& socket)
{
    if (initialized_) {
        return true;
    }

    host_ = host;
    user_ = user;
    pass_ = pass;
    db_ = db;
    port_ = port;
    socket_ = socket;

    // Create initial connections
    for (size_t i = 0; i < poolSize_; ++i) {
        auto connection = createConnection();
        if (!connection) {
            std::cout << "[Error - ConnectionPool::initialize] Failed to create initial connection " << i + 1 << std::endl;
            shutdown();
            return false;
        }
        availableConnections_.push(connection);
        ++totalConnections_; // Count initial connections created
    }

    initialized_ = true;

    // Start cleanup thread
    cleanupThread_ = std::thread(&ConnectionPool::cleanupTask, this);

    std::cout << "[Info - ConnectionPool::initialize] Connection pool initialized with " << poolSize_ << " connections" << std::endl;
    return true;
}

std::shared_ptr<ConnectionPool::Connection> ConnectionPool::acquire()
{
    if (shutdown_ || !initialized_) {
        return nullptr;
    }

    std::unique_lock<std::mutex> lock(poolMutex_);

    // Wait for available connection or timeout
    if (!poolCondition_.wait_for(lock, acquireTimeout_, [this]() {
        return shutdown_ || !availableConnections_.empty() || totalConnections_ < maxPoolSize_;
    })) {
        std::cout << "[Warning - ConnectionPool::acquire] Timeout waiting for connection (total: "
                  << totalConnections_ << ", available: " << availableConnections_.size()
                  << ", max: " << maxPoolSize_ << ")" << std::endl;
        ++stats_.failedAcquires;
        return nullptr;
    }

    if (shutdown_) {
        return nullptr;
    }

    std::shared_ptr<Connection> connection;

    // Try to get from available connections
    if (!availableConnections_.empty()) {
        connection = availableConnections_.front();
        availableConnections_.pop();
    } else if (totalConnections_ < maxPoolSize_) {
        // Create new connection if under max limit
        connection = createConnection();
        if (!connection) {
            std::cout << "[Error - ConnectionPool::acquire] Failed to create new connection" << std::endl;
            ++stats_.failedAcquires;
            return nullptr;
        }
        ++totalConnections_; // Count new connection created
    } else {
        // This shouldn't happen due to the wait condition, but just in case
        ++stats_.failedAcquires;
        return nullptr;
    }

    // Validate connection (thorough check if idle for a while)
    bool needsThoroughValidation = (std::chrono::steady_clock::now() - connection->lastUsed) > idleValidationThreshold_;
    if (!validateConnection(connection, needsThoroughValidation)) {
        std::cout << "[Warning - ConnectionPool::acquire] Connection validation failed, creating new connection" << std::endl;
        ++stats_.validationFailures;
        // Decrement count for the invalid connection
        if (totalConnections_ > 0) {
            --totalConnections_;
        }
        connection = createConnection();
        if (!connection) {
            ++stats_.failedAcquires;
            return nullptr;
        }
        ++totalConnections_; // Count replacement connection
        ++stats_.reconnections;
    }

    // Final safety check - ensure handle is valid before handing out
    if (!connection->handle) {
        std::cout << "[Error - ConnectionPool::acquire] Connection passed validation but handle is NULL" << std::endl;
        ++stats_.failedAcquires;
        if (totalConnections_ > 0) {
            --totalConnections_;
        }
        return nullptr;
    }

    connection->inUse = true;
    connection->lastUsed = std::chrono::steady_clock::now();
    ++inUseConnections_; // Count connection as now in use
    ++stats_.totalAcquires;

    return connection;
}

void ConnectionPool::release(std::shared_ptr<ConnectionPool::Connection> connection)
{
    if (!connection || shutdown_) {
        return;
    }

    std::lock_guard<std::mutex> lock(poolMutex_);

    connection->inUse = false;
    connection->lastUsed = std::chrono::steady_clock::now();
    if (inUseConnections_ > 0) {
        --inUseConnections_; // Connection is no longer in use
    }
    ++stats_.totalReleases;

    bool isValid = validateConnection(connection, false);  // Quick validation on release
    if (isValid) {
        // Always add valid connections back to the available pool
        // The pool will naturally shrink if connections are invalidated elsewhere
        availableConnections_.push(connection);
    } else {
        // Attempt to reconnect invalid connection
        if (connection->connect(host_, user_, pass_, db_, port_, socket_)) {
            std::cout << "[Info - ConnectionPool::release] Successfully reconnected invalid connection" << std::endl;
            availableConnections_.push(connection);
            ++stats_.reconnections;
        } else {
            // Connection is invalid and cannot be reconnected, discard it
            std::cout << "[Warning - ConnectionPool::release] Discarding invalid connection (handle: "
                      << (connection->handle ? "valid" : "null") << ", connected: "
                      << (connection->isConnected() ? "yes" : "no") << ")" << std::endl;
            if (totalConnections_ > 0) {
                --totalConnections_;
            }
            ++stats_.validationFailures;
        }
    }

    poolCondition_.notify_one();
}

void ConnectionPool::cleanup()
{
    if (shutdown_) {
        return;
    }

    std::lock_guard<std::mutex> lock(poolMutex_);
    auto now = std::chrono::steady_clock::now();

    // Remove expired connections from available pool, but maintain at least minPoolSize_ connections
    std::queue<std::shared_ptr<Connection>> tempQueue;
    size_t keptConnections = 0;

    while (!availableConnections_.empty()) {
        auto connection = availableConnections_.front();
        availableConnections_.pop();

        bool isExpired = (now - connection->lastUsed) >= connectionTimeout_;
        bool needToKeep = keptConnections < minPoolSize_;

        if (!isExpired || needToKeep) {
            // Keep connection if it's not expired, or if we need to maintain minimum pool size
            tempQueue.push(connection);
            ++keptConnections;
        } else {
            if (totalConnections_ > 0) {
                --totalConnections_;
            }
        }
    }

    // If we have fewer connections than minimum, create new ones
    while (tempQueue.size() < minPoolSize_ && totalConnections_ < maxPoolSize_) {
        auto connection = createConnection();
        if (connection) {
            tempQueue.push(connection);
            ++totalConnections_;
            std::cout << "[Info - ConnectionPool::cleanup] Created connection to maintain minimum pool size" << std::endl;
        } else {
            break;  // Failed to create connection, stop trying
        }
    }

    availableConnections_ = std::move(tempQueue);
}

void ConnectionPool::shutdown()
{
    if (shutdown_) {
        return;
    }

    shutdown_ = true;

    // Notify cleanup thread to wake up immediately
    poolCondition_.notify_all();

    // Stop cleanup thread
    if (cleanupThread_.joinable()) {
        cleanupThread_.join();
    }

    // Clear all connections
    {
        std::lock_guard<std::mutex> lock(poolMutex_);
        while (!availableConnections_.empty()) {
            availableConnections_.pop();
        }
        totalConnections_ = 0;
        inUseConnections_ = 0;
    }

    initialized_ = false;

    std::cout << "[Info - ConnectionPool::shutdown] Connection pool shut down" << std::endl;
}

std::shared_ptr<ConnectionPool::Connection> ConnectionPool::createConnection()
{
    auto connection = std::make_shared<Connection>();
    if (!connection->connect(host_, user_, pass_, db_, port_, socket_)) {
        return nullptr;
    }
    return connection;
}

bool ConnectionPool::validateConnection(std::shared_ptr<Connection> connection, bool thorough)
{
    if (!connection || !connection->isConnected()) {
        return false;
    }

    // For quick validation, just check if handle is valid
    if (!thorough) {
        return connection->handle != nullptr;
    }

    // Thorough validation - actually ping the server
    return connection->ping();
}

size_t ConnectionPool::getActiveConnections() const
{
    std::lock_guard<std::mutex> lock(poolMutex_);
    return inUseConnections_.load();
}

size_t ConnectionPool::getTotalConnections() const
{
    std::lock_guard<std::mutex> lock(poolMutex_);
    return totalConnections_.load();
}

size_t ConnectionPool::getAvailableConnections() const
{
    std::lock_guard<std::mutex> lock(poolMutex_);
    return availableConnections_.size();
}

ConnectionPool::PoolSnapshot ConnectionPool::getSnapshot() const
{
    std::lock_guard<std::mutex> lock(poolMutex_);
    return PoolSnapshot{
        poolSize_,
        minPoolSize_,
        maxPoolSize_,
        totalConnections_.load(),
        inUseConnections_.load(),
        availableConnections_.size(),
        stats_.totalAcquires.load(),
        stats_.totalReleases.load(),
        stats_.failedAcquires.load(),
        stats_.reconnections.load(),
        stats_.validationFailures.load()
    };
}

bool ConnectionPool::isRecoverableError(unsigned int errorCode)
{
    return errorCode == CR_SERVER_LOST ||
           errorCode == CR_SERVER_GONE_ERROR ||
           errorCode == CR_CONN_HOST_ERROR ||
           errorCode == 1053 /*ER_SERVER_SHUTDOWN*/ ||
           errorCode == CR_CONNECTION_ERROR;
}

void ConnectionPool::cleanupTask()
{
    std::unique_lock<std::mutex> lock(poolMutex_);
    while (!shutdown_.load()) {
        // Wait for 5 minutes or until shutdown is signaled
        poolCondition_.wait_for(lock, std::chrono::minutes(5), [this]() {
            return shutdown_.load();
        });

        if (!shutdown_.load()) {
            // Temporarily unlock while cleaning up to avoid blocking other operations
            lock.unlock();
            cleanup();
            lock.lock();
        }
    }
}
