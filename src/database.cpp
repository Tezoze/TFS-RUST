// Copyright 2022 The Forgotten Server Authors. All rights reserved.
// Use of this source code is governed by the GPL-2.0 License that can be found in the LICENSE file.

#include "otpch.h"

#include "database.h"
#include "connectionpool.h"
#include "configmanager.h"

#include <mysql/errmsg.h>
#include <unordered_map>
#include <vector>

extern ConfigManager g_config;

// Thread-local cleanup helper to prevent connection leaks
struct ThreadLocalCleanup {
	~ThreadLocalCleanup();
};

// Thread-safe transaction state management
class TransactionManager
{
public:
	static TransactionManager& getInstance()
	{
		static TransactionManager instance;
		return instance;
	}

	std::shared_ptr<ConnectionPool::Connection> getTransactionConnection()
	{
		ensureThreadCleanup(); // Ensure cleanup is registered for this thread
		std::lock_guard<std::mutex> lock(mutex_);
		auto it = transactionConnections_.find(std::this_thread::get_id());
		return (it != transactionConnections_.end()) ? it->second : nullptr;
	}

	void setTransactionConnection(std::shared_ptr<ConnectionPool::Connection> connection)
	{
		ensureThreadCleanup(); // Ensure cleanup is registered for this thread
		std::lock_guard<std::mutex> lock(mutex_);
		if (connection) {
			transactionConnections_[std::this_thread::get_id()] = connection;
		} else {
			transactionConnections_.erase(std::this_thread::get_id());
		}
	}

	int getTransactionDepth()
	{
		std::lock_guard<std::mutex> lock(mutex_);
		auto it = transactionDepths_.find(std::this_thread::get_id());
		return (it != transactionDepths_.end()) ? it->second : 0;
	}

	void setTransactionDepth(int depth)
	{
		std::lock_guard<std::mutex> lock(mutex_);
		if (depth > 0) {
			transactionDepths_[std::this_thread::get_id()] = depth;
		} else {
			transactionDepths_.erase(std::this_thread::get_id());
		}
	}

	// Generate unique savepoint name for nested transactions
	std::string getSavepointName(int depth)
	{
		return "sp_" + std::to_string(depth);
	}

	void cleanupThread()
	{
		std::lock_guard<std::mutex> lock(mutex_);
		auto threadId = std::this_thread::get_id();

		// Check for potential leaks before cleanup
		auto connIt = transactionConnections_.find(threadId);
		auto depthIt = transactionDepths_.find(threadId);

		std::cout << "[Debug - TransactionManager::cleanupThread] Thread cleanup called for thread " << threadId << std::endl;

		if (connIt != transactionConnections_.end() && connIt->second) {
			auto connection = connIt->second;
			int depth = (depthIt != transactionDepths_.end() ? depthIt->second : 0);
			
			std::cout << "[Warning - TransactionManager::cleanupThread] Thread " << threadId
					  << " terminated with active transaction connection (depth: " << depth
					  << "). Rolling back and releasing connection." << std::endl;
			
			// Rollback any uncommitted transaction
			if (depth > 0 && connection->handle) {
				try {
					mysql_rollback(connection->handle);
					std::cout << "[Debug - TransactionManager::cleanupThread] Rollback completed" << std::endl;
				} catch (...) {
					std::cout << "[Error - TransactionManager::cleanupThread] Exception during rollback" << std::endl;
				}
			}
			
			// Release the connection back to the pool via Database singleton
			auto& db = Database::getInstance();
			if (db.connectionPool) {
				std::cout << "[Debug - TransactionManager::cleanupThread] Releasing connection back to pool" << std::endl;
				db.connectionPool->release(connection);
				std::cout << "[Debug - TransactionManager::cleanupThread] Connection released" << std::endl;
			}
		}

		transactionConnections_.erase(threadId);
		transactionDepths_.erase(threadId);
		std::cout << "[Debug - TransactionManager::cleanupThread] Thread cleanup completed" << std::endl;
	}

	size_t getActiveTransactionCount() const
	{
		std::lock_guard<std::mutex> lock(mutex_);
		return transactionConnections_.size();
	}

	void logTransactionStats() const
	{
		std::lock_guard<std::mutex> lock(mutex_);
		size_t activeTransactions = transactionConnections_.size();
		if (activeTransactions > 0) {
			std::cout << "[Info - TransactionManager] Active transactions: " << activeTransactions << std::endl;
			for (const auto& pair : transactionDepths_) {
				if (pair.second > 0) {
					std::cout << "[Info - TransactionManager] Thread " << pair.first
							  << " has transaction depth: " << pair.second << std::endl;
				}
			}
		}
	}

private:
	TransactionManager() = default;
	~TransactionManager() = default;

	// non-copyable
	TransactionManager(const TransactionManager&) = delete;
	TransactionManager& operator=(const TransactionManager&) = delete;

	void ensureThreadCleanup()
	{
		// Thread-local cleanup object will automatically call cleanupThread() when thread exits
		static thread_local ThreadLocalCleanup cleanup;
		(void)cleanup; // Suppress unused variable warning
	}

	mutable std::mutex mutex_;
	std::unordered_map<std::thread::id, std::shared_ptr<ConnectionPool::Connection>> transactionConnections_;
	std::unordered_map<std::thread::id, int> transactionDepths_;
};

// Thread-local cleanup implementation
ThreadLocalCleanup::~ThreadLocalCleanup()
{
	TransactionManager::getInstance().cleanupThread();
}

// Private helper methods for connection management
namespace {
	std::shared_ptr<ConnectionPool::Connection> getConnection(const std::unique_ptr<ConnectionPool>& connectionPool)
	{
		if (!connectionPool) {
			return nullptr;
		}

		auto& transactionManager = TransactionManager::getInstance();
		auto transactionConnection = transactionManager.getTransactionConnection();
		int transactionDepth = transactionManager.getTransactionDepth();

		// If we're in a transaction, use the transaction connection
		if (transactionDepth > 0 && transactionConnection) {
			return transactionConnection;
		}

		// Otherwise, acquire a new connection from the pool
		return const_cast<ConnectionPool*>(connectionPool.get())->acquire();
	}

	void releaseConnection(const std::unique_ptr<ConnectionPool>& connectionPool, std::shared_ptr<ConnectionPool::Connection> connection)
	{
		if (!connectionPool || !connection) {
			return;
		}

		auto& transactionManager = TransactionManager::getInstance();
		auto transactionConnection = transactionManager.getTransactionConnection();
		int transactionDepth = transactionManager.getTransactionDepth();

		// Don't release transaction connections - they're managed by the transaction methods
		if (transactionDepth > 0 && connection == transactionConnection) {
			return;
		}

		// Release regular connections back to the pool
		const_cast<ConnectionPool*>(connectionPool.get())->release(connection);
	}

	// RAII wrapper for database connections to prevent memory leaks
	class ConnectionGuard {
	public:
		ConnectionGuard(const std::unique_ptr<ConnectionPool>& pool)
			: connectionPool_(pool), connection_(getConnection(pool)) {}

		~ConnectionGuard() {
			if (connection_) {
				releaseConnection(connectionPool_, connection_);
			}
		}

		// Disable copy and move
		ConnectionGuard(const ConnectionGuard&) = delete;
		ConnectionGuard& operator=(const ConnectionGuard&) = delete;
		ConnectionGuard(ConnectionGuard&&) = delete;
		ConnectionGuard& operator=(ConnectionGuard&&) = delete;

		std::shared_ptr<ConnectionPool::Connection> get() const {
			return connection_;
		}

		explicit operator bool() const {
			return connection_ != nullptr;
		}

	private:
		const std::unique_ptr<ConnectionPool>& connectionPool_;
		std::shared_ptr<ConnectionPool::Connection> connection_;
	};
}

Database::Database() = default;

Database::~Database()
{
	// Clean up single connection if used
	if (singleConnection) {
		mysql_close(singleConnection);
		singleConnection = nullptr;
	}
}

bool Database::connect()
{
	useConnectionPool = g_config.getBoolean(ConfigManager::MYSQL_CONNECTION_POOL_ENABLED);

	if (useConnectionPool) {
		// Connection pooling mode
		if (connectionPool) {
			return true; // Already connected
		}

		size_t poolSize = static_cast<size_t>(g_config.getNumber(ConfigManager::MYSQL_CONNECTION_POOL_SIZE));
		size_t maxPoolSize = static_cast<size_t>(g_config.getNumber(ConfigManager::MYSQL_CONNECTION_MAX_POOL_SIZE));
		size_t minPoolSize = static_cast<size_t>(g_config.getNumber(ConfigManager::MYSQL_CONNECTION_MIN_POOL_SIZE));
		std::chrono::seconds connectionTimeout(g_config.getNumber(ConfigManager::MYSQL_CONNECTION_TIMEOUT));
		std::chrono::seconds acquireTimeout(g_config.getNumber(ConfigManager::MYSQL_CONNECTION_ACQUIRE_TIMEOUT));

		connectionPool = std::make_unique<ConnectionPool>(poolSize, maxPoolSize, connectionTimeout, acquireTimeout, minPoolSize);

		bool result = connectionPool->initialize(
			g_config.getString(ConfigManager::MYSQL_HOST),
			g_config.getString(ConfigManager::MYSQL_USER),
			g_config.getString(ConfigManager::MYSQL_PASS),
			g_config.getString(ConfigManager::MYSQL_DB),
			static_cast<unsigned int>(g_config.getNumber(ConfigManager::SQL_PORT)),
			g_config.getString(ConfigManager::MYSQL_SOCK)
		);

		if (result) {
			std::cout << "[Info - Database::connect] Connection pooling ENABLED" << std::endl;
		}
		return result;
	} else {
		// Single connection mode (pooling disabled)
		if (singleConnection) {
			return true; // Already connected
		}

		singleConnection = mysql_init(nullptr);
		if (!singleConnection) {
			std::cout << "[Error - Database::connect] Failed to initialize MySQL connection" << std::endl;
			return false;
		}

		// Enable automatic reconnect - critical for handling MySQL connection timeouts
		// Without this, the server will fail after MySQL's wait_timeout expires
		bool reconnect = true;
		mysql_options(singleConnection, MYSQL_OPT_RECONNECT, &reconnect);

		// Disable SSL verification (matches Canary behavior)

		if (!mysql_real_connect(singleConnection,
								g_config.getString(ConfigManager::MYSQL_HOST).c_str(),
								g_config.getString(ConfigManager::MYSQL_USER).c_str(),
								g_config.getString(ConfigManager::MYSQL_PASS).c_str(),
								g_config.getString(ConfigManager::MYSQL_DB).c_str(),
								static_cast<unsigned int>(g_config.getNumber(ConfigManager::SQL_PORT)),
								g_config.getString(ConfigManager::MYSQL_SOCK).c_str(),
								0)) {
			std::cout << "[Error - Database::connect] Failed to connect to database: " << mysql_error(singleConnection) << std::endl;
			mysql_close(singleConnection);
			singleConnection = nullptr;
			return false;
		}

		std::cout << "[Info - Database::connect] Connection pooling DISABLED (using single connection)" << std::endl;
		return true;
	}
}

bool Database::reconnect()
{
	if (useConnectionPool) {
		// Connection pool handles its own reconnection
		return true;
	}

	std::cout << "[Info - Database::reconnect] Attempting to reconnect to MySQL..." << std::endl;

	// Close existing connection if any
	if (singleConnection) {
		mysql_close(singleConnection);
		singleConnection = nullptr;
	}

	// Reinitialize connection
	singleConnection = mysql_init(nullptr);
	if (!singleConnection) {
		std::cout << "[Error - Database::reconnect] Failed to initialize MySQL connection" << std::endl;
		return false;
	}

	// Enable automatic reconnect
	bool reconnectOpt = true;
	mysql_options(singleConnection, MYSQL_OPT_RECONNECT, &reconnectOpt);

	// Disable SSL verification

	if (!mysql_real_connect(singleConnection,
							g_config.getString(ConfigManager::MYSQL_HOST).c_str(),
							g_config.getString(ConfigManager::MYSQL_USER).c_str(),
							g_config.getString(ConfigManager::MYSQL_PASS).c_str(),
							g_config.getString(ConfigManager::MYSQL_DB).c_str(),
							static_cast<unsigned int>(g_config.getNumber(ConfigManager::SQL_PORT)),
							g_config.getString(ConfigManager::MYSQL_SOCK).c_str(),
							0)) {
		std::cout << "[Error - Database::reconnect] Failed to reconnect: " << mysql_error(singleConnection) << std::endl;
		mysql_close(singleConnection);
		singleConnection = nullptr;
		return false;
	}

	std::cout << "[Info - Database::reconnect] Successfully reconnected to MySQL" << std::endl;
	return true;
}

bool Database::beginTransaction()
{
	if (useConnectionPool) {
		if (!connectionPool) {
			return false;
		}
	} else {
		// Single connection mode - transactions work directly on the single connection
		if (!singleConnection) {
			return false;
		}

		// For single connection, just execute BEGIN directly
		if (mysql_real_query(singleConnection, "START TRANSACTION", 17) != 0) {
			std::cout << "[Error - Database::beginTransaction] Failed to start transaction: " << mysql_error(singleConnection) << std::endl;
			return false;
		}
		return true;
	}

	auto& transactionManager = TransactionManager::getInstance();
	auto transactionConnection = transactionManager.getTransactionConnection();
	int transactionDepth = transactionManager.getTransactionDepth();

	// Cast away const for transaction operations
	auto& pool = const_cast<std::unique_ptr<ConnectionPool>&>(connectionPool);

	try {
		// If this is the first transaction level, acquire a dedicated connection
		if (transactionDepth == 0) {
			transactionConnection = pool->acquire();
			if (!transactionConnection) {
				std::cout << "[Error - Database::beginTransaction] Failed to acquire database connection" << std::endl;
				return false;
			}

			// Start transaction on this connection
			if (mysql_real_query(transactionConnection->handle, "BEGIN", 5) != 0) {
				std::cout << "[Error - mysql_real_query] Query: BEGIN" << std::endl;
				std::cout << "Message: " << mysql_error(transactionConnection->handle) << std::endl;
				pool->release(transactionConnection);
				return false;
			}

			transactionManager.setTransactionConnection(transactionConnection);
		} else {
			// Nested transaction - use unique savepoint name based on depth
			std::string savepointQuery = "SAVEPOINT " + transactionManager.getSavepointName(transactionDepth + 1);
			if (mysql_real_query(transactionConnection->handle, savepointQuery.c_str(), savepointQuery.length()) != 0) {
				std::cout << "[Error - mysql_real_query] Query: " << savepointQuery << std::endl;
				std::cout << "Message: " << mysql_error(transactionConnection->handle) << std::endl;
				return false;
			}
		}

		transactionManager.setTransactionDepth(transactionDepth + 1);
		return true;
	} catch (const std::exception& e) {
		std::cout << "[Critical - Database::beginTransaction] Exception: " << e.what() << std::endl;
		// Clean up on exception
		if (transactionDepth == 0 && transactionConnection) {
			pool->release(transactionConnection);
			transactionManager.setTransactionConnection(nullptr);
		}
		return false;
	} catch (...) {
		std::cout << "[Critical - Database::beginTransaction] Unknown exception" << std::endl;
		// Clean up on exception
		if (transactionDepth == 0 && transactionConnection) {
			pool->release(transactionConnection);
			transactionManager.setTransactionConnection(nullptr);
		}
		return false;
	}
}

bool Database::rollback()
{
	if (!useConnectionPool) {
		// Single connection mode
		if (!singleConnection) {
			return false;
		}

		if (mysql_rollback(singleConnection) != 0) {
			std::cout << "[Error - Database::rollback] Failed to rollback transaction: " << mysql_error(singleConnection) << std::endl;
			return false;
		}
		return true;
	}

	auto& transactionManager = TransactionManager::getInstance();
	auto transactionConnection = transactionManager.getTransactionConnection();
	int transactionDepth = transactionManager.getTransactionDepth();

	if (!connectionPool || transactionDepth == 0 || !transactionConnection) {
		return false;
	}

	// Cast away const for transaction operations
	auto& pool = const_cast<std::unique_ptr<ConnectionPool>&>(connectionPool);

	try {
		bool success = true;
		if (transactionDepth == 1) {
			// Outer transaction - full rollback
			if (mysql_rollback(transactionConnection->handle) != 0) {
				std::cout << "[Error - mysql_rollback] Message: " << mysql_error(transactionConnection->handle) << std::endl;
				success = false;
			}
		} else {
			// Nested transaction - rollback to savepoint with unique name
			std::string rollbackQuery = "ROLLBACK TO SAVEPOINT " + transactionManager.getSavepointName(transactionDepth);
			if (mysql_real_query(transactionConnection->handle, rollbackQuery.c_str(), rollbackQuery.length()) != 0) {
				std::cout << "[Error - mysql_real_query] Query: " << rollbackQuery << std::endl;
				std::cout << "Message: " << mysql_error(transactionConnection->handle) << std::endl;
				success = false;
			}
		}

		int newDepth = transactionDepth - 1;
		transactionManager.setTransactionDepth(newDepth);

		// If this was the last transaction level, release the connection
		if (newDepth == 0) {
			pool->release(transactionConnection);
			transactionManager.setTransactionConnection(nullptr);
		}

		return success;
	} catch (const std::exception& e) {
		std::cout << "[Critical - Database::rollback] Exception: " << e.what() << std::endl;
		// Attempt emergency cleanup
		try {
			transactionManager.setTransactionDepth(0);
			if (transactionConnection) {
				pool->release(transactionConnection);
				transactionManager.setTransactionConnection(nullptr);
			}
		} catch (...) {
			// If cleanup fails, we're in a bad state but at least we logged the original error
		}
		return false;
	} catch (...) {
		std::cout << "[Critical - Database::rollback] Unknown exception" << std::endl;
		// Attempt emergency cleanup
		try {
			transactionManager.setTransactionDepth(0);
			if (transactionConnection) {
				pool->release(transactionConnection);
				transactionManager.setTransactionConnection(nullptr);
			}
		} catch (...) {
			// If cleanup fails, we're in a bad state but at least we logged the original error
		}
		return false;
	}
}

bool Database::commit()
{
	if (!useConnectionPool) {
		// Single connection mode
		if (!singleConnection) {
			return false;
		}

		if (mysql_commit(singleConnection) != 0) {
			std::cout << "[Error - Database::commit] Failed to commit transaction: " << mysql_error(singleConnection) << std::endl;
			return false;
		}
		return true;
	}

	auto& transactionManager = TransactionManager::getInstance();
	auto transactionConnection = transactionManager.getTransactionConnection();
	int transactionDepth = transactionManager.getTransactionDepth();

	if (!connectionPool || transactionDepth == 0 || !transactionConnection) {
		return false;
	}

	// Cast away const for transaction operations
	auto& pool = const_cast<std::unique_ptr<ConnectionPool>&>(connectionPool);

	try {
		bool success = true;
		if (transactionDepth == 1) {
			// Outer transaction - full commit
			if (mysql_commit(transactionConnection->handle) != 0) {
				std::cout << "[Error - mysql_commit] Message: " << mysql_error(transactionConnection->handle) << std::endl;
				success = false;
			}
		} else {
			// Nested transaction - release savepoint with unique name
			std::string releaseQuery = "RELEASE SAVEPOINT " + transactionManager.getSavepointName(transactionDepth);
			if (mysql_real_query(transactionConnection->handle, releaseQuery.c_str(), releaseQuery.length()) != 0) {
				std::cout << "[Error - mysql_real_query] Query: " << releaseQuery << std::endl;
				std::cout << "Message: " << mysql_error(transactionConnection->handle) << std::endl;
				success = false;
			}
		}

		int newDepth = transactionDepth - 1;
		transactionManager.setTransactionDepth(newDepth);

		// If this was the last transaction level, release the connection
		if (newDepth == 0) {
			pool->release(transactionConnection);
			transactionManager.setTransactionConnection(nullptr);
		}

		return success;
	} catch (const std::exception& e) {
		std::cout << "[Critical - Database::commit] Exception: " << e.what() << std::endl;
		// Attempt emergency cleanup - rollback on commit failure
		try {
			transactionManager.setTransactionDepth(0);
			if (transactionConnection) {
				pool->release(transactionConnection);
				transactionManager.setTransactionConnection(nullptr);
			}
		} catch (...) {
			// If cleanup fails, we're in a bad state but at least we logged the original error
		}
		return false;
	} catch (...) {
		std::cout << "[Critical - Database::commit] Unknown exception" << std::endl;
		// Attempt emergency cleanup - rollback on commit failure
		try {
			transactionManager.setTransactionDepth(0);
			if (transactionConnection) {
				pool->release(transactionConnection);
				transactionManager.setTransactionConnection(nullptr);
			}
		} catch (...) {
			// If cleanup fails, we're in a bad state but at least we logged the original error
		}
		return false;
	}
}

bool Database::executeQuery(const std::string& query)
{
	MYSQL* handle = nullptr;

	if (useConnectionPool) {
		ConnectionGuard guard(connectionPool);
		auto connection = guard.get();
		if (!connection) {
			std::cout << "[Error - Database::executeQuery] Failed to acquire database connection" << std::endl;
			return false;
		}
		handle = connection->handle;
		if (!handle) {
			std::cout << "[Error - Database::executeQuery] Connection has NULL MySQL handle" << std::endl;
			return false;
		}

		bool success = true;
		int retryCount = 0;
		const int MAX_RETRIES = 10;

		while (mysql_real_query(handle, query.c_str(), query.length()) != 0) {
			std::cout << "[Error - mysql_real_query] Query: " << query.substr(0, 256) << std::endl << "Message: " << mysql_error(handle) << std::endl;
			auto error = mysql_errno(handle);

			if (++retryCount > MAX_RETRIES) {
				std::cout << "[Error] Max retries exceeded for query." << std::endl;
				success = false;
				break;
			}

			if (!ConnectionPool::isRecoverableError(error)) {
				success = false;
				break;
			}
			std::this_thread::sleep_for(std::chrono::seconds(1));
		}

		MYSQL_RES* m_res = mysql_store_result(handle);

		if (m_res) {
			mysql_free_result(m_res);
		}

		return success;
	} else {
		// Single connection mode
		if (!singleConnection) {
			std::cout << "[Error - Database::executeQuery] No database connection" << std::endl;
			return false;
		}
		handle = singleConnection;

		// Validate connection is still alive before query
		if (mysql_ping(handle) != 0) {
			std::cout << "[Warning - Database::executeQuery] Connection lost, attempting reconnect..." << std::endl;
			if (!reconnect()) {
				std::cout << "[Error - Database::executeQuery] Failed to reconnect" << std::endl;
				return false;
			}
			handle = singleConnection;
		}

		bool success = true;
		int retryCount = 0;
		const int MAX_RETRIES = 10;

		while (mysql_real_query(handle, query.c_str(), query.length()) != 0) {
			std::cout << "[Error - mysql_real_query] Query: " << query.substr(0, 256) << std::endl << "Message: " << mysql_error(handle) << std::endl;
			auto error = mysql_errno(handle);

			if (!ConnectionPool::isRecoverableError(error)) {
				success = false;
				break;
			}

			if (++retryCount > MAX_RETRIES) {
				std::cout << "[Error] Max retries exceeded for query." << std::endl;
				success = false;
				break;
			}

			// Attempt explicit reconnect
			if (reconnect()) {
				handle = singleConnection;
			}

			std::this_thread::sleep_for(std::chrono::seconds(1));
		}

		MYSQL_RES* m_res = mysql_store_result(handle);

		if (m_res) {
			mysql_free_result(m_res);
		}

		return success;
	}
}

DBResult_ptr Database::storeQuery(const std::string& query)
{
	MYSQL* handle = nullptr;

	if (useConnectionPool) {
		ConnectionGuard guard(connectionPool);
		auto connection = guard.get();
		if (!connection) {
			std::cout << "[Error - Database::storeQuery] Failed to acquire database connection" << std::endl;
			return nullptr;
		}
		handle = connection->handle;
		if (!handle) {
			std::cout << "[Error - Database::storeQuery] Connection has NULL MySQL handle" << std::endl;
			return nullptr;
		}

		int retryCount = 0;
		const int MAX_RETRIES = 10;

		// Query execution with retry loop
		while (true) {
			if (mysql_real_query(handle, query.c_str(), query.length()) == 0) {
				break;  // Query succeeded
			}

			std::cout << "[Error - mysql_real_query] Query: " << query << std::endl << "Message: " << mysql_error(handle) << std::endl;
			auto error = mysql_errno(handle);

			if (++retryCount > MAX_RETRIES) {
				std::cout << "[Error] Max retries exceeded for query." << std::endl;
				return nullptr;
			}

			if (!ConnectionPool::isRecoverableError(error)) {
				return nullptr;
			}
			std::this_thread::sleep_for(std::chrono::seconds(1));
		}

		// Store result with retry loop
		retryCount = 0;
		MYSQL_RES* res = nullptr;
		while (true) {
			res = mysql_store_result(handle);
			if (res != nullptr) {
				break;  // Got result
			}

			std::cout << "[Error - mysql_store_result] Query: " << query << std::endl << "Message: " << mysql_error(handle) << std::endl;
			auto error = mysql_errno(handle);

			if (!ConnectionPool::isRecoverableError(error)) {
				return nullptr;
			}

			if (++retryCount > MAX_RETRIES) {
				std::cout << "[Error] Max retries exceeded for store_result." << std::endl;
				return nullptr;
			}

			std::this_thread::sleep_for(std::chrono::seconds(1));

			// Re-execute query after connection recovery
			if (mysql_real_query(handle, query.c_str(), query.length()) != 0) {
				auto queryError = mysql_errno(handle);
				if (!ConnectionPool::isRecoverableError(queryError)) {
					return nullptr;
				}
			}
		}

		// retrieving results of query
		DBResult_ptr result = std::make_shared<DBResult>(res);

		if (!result->hasNext()) {
			return nullptr;
		}

		return result;
	} else {
		// Single connection mode
		if (!singleConnection) {
			std::cout << "[Error - Database::storeQuery] No database connection" << std::endl;
			return nullptr;
		}
		handle = singleConnection;

		// Validate connection is still alive before query
		if (mysql_ping(handle) != 0) {
			std::cout << "[Warning - Database::storeQuery] Connection lost, attempting reconnect..." << std::endl;
			if (!reconnect()) {
				std::cout << "[Error - Database::storeQuery] Failed to reconnect" << std::endl;
				return nullptr;
			}
			handle = singleConnection;
		}

		int retryCount = 0;
		const int MAX_RETRIES = 10;

		// Query execution with retry loop
		while (true) {
			if (mysql_real_query(handle, query.c_str(), query.length()) == 0) {
				break;  // Query succeeded
			}

			std::cout << "[Error - mysql_real_query] Query: " << query << std::endl << "Message: " << mysql_error(handle) << std::endl;
			auto error = mysql_errno(handle);

			if (!ConnectionPool::isRecoverableError(error)) {
				return nullptr;
			}

			if (++retryCount > MAX_RETRIES) {
				std::cout << "[Error] Max retries exceeded for query." << std::endl;
				return nullptr;
			}

			// Attempt explicit reconnect
			if (reconnect()) {
				handle = singleConnection;
			}

			std::this_thread::sleep_for(std::chrono::seconds(1));
		}

		// Store result with retry loop
		retryCount = 0;
		MYSQL_RES* res = nullptr;
		while (true) {
			res = mysql_store_result(handle);
			if (res != nullptr) {
				break;  // Got result
			}

			std::cout << "[Error - mysql_store_result] Query: " << query << std::endl << "Message: " << mysql_error(handle) << std::endl;
			auto error = mysql_errno(handle);

			if (!ConnectionPool::isRecoverableError(error)) {
				return nullptr;
			}

			if (++retryCount > MAX_RETRIES) {
				std::cout << "[Error] Max retries exceeded for store_result." << std::endl;
				return nullptr;
			}

			// Attempt explicit reconnect
			if (reconnect()) {
				handle = singleConnection;
			}

			std::this_thread::sleep_for(std::chrono::seconds(1));

			// Re-execute query after reconnect
			if (mysql_real_query(handle, query.c_str(), query.length()) != 0) {
				auto queryError = mysql_errno(handle);
				if (!ConnectionPool::isRecoverableError(queryError)) {
					return nullptr;
				}
			}
		}

		// retrieving results of query
		DBResult_ptr result = std::make_shared<DBResult>(res);

		if (!result->hasNext()) {
			return nullptr;
		}

		return result;
	}
}

std::string Database::escapeString(const std::string& s) const
{
	return escapeBlob(s.c_str(), s.length());
}

std::string Database::escapeBlob(const char* s, uint32_t length) const
{
	MYSQL* handle = nullptr;

	if (useConnectionPool) {
		ConnectionGuard guard(connectionPool);
		auto connection = guard.get();
		if (!connection) {
			return std::string();
		}
		handle = connection->handle;
	} else {
		// Single connection mode
		if (!singleConnection) {
			return std::string();
		}
		handle = singleConnection;
	}

	// the worst case is 2n + 1
	size_t maxLength = (length * 2) + 1;

	std::string escaped;
	escaped.reserve(maxLength + 2);
	escaped.push_back('\'');

	if (length != 0) {
		// Use vector for exception-safe memory management
		std::vector<char> output(maxLength);
		mysql_real_escape_string(handle, output.data(), s, length);
		escaped.append(output.data());
	}

	escaped.push_back('\'');

	return escaped;
}

uint64_t Database::getLastInsertId() const
{
	if (useConnectionPool) {
		auto& transactionManager = TransactionManager::getInstance();
		auto transactionConnection = transactionManager.getTransactionConnection();
		int transactionDepth = transactionManager.getTransactionDepth();

		// If we're in a transaction, use the transaction connection
		if (transactionDepth > 0 && transactionConnection) {
			return static_cast<uint64_t>(mysql_insert_id(transactionConnection->handle));
		}

		// Otherwise, this is unreliable since we don't know which connection did the last INSERT
		// Return 0 to indicate the ID is not available
		return 0;
	} else {
		// Single connection mode - reliable
		if (singleConnection) {
			return static_cast<uint64_t>(mysql_insert_id(singleConnection));
		}
		return 0;
	}
}

size_t Database::getPoolSize() const
{
	return connectionPool ? connectionPool->getPoolSize() : 0;
}

size_t Database::getActiveConnections() const
{
	return connectionPool ? connectionPool->getActiveConnections() : 0;
}

size_t Database::getAvailableConnections() const
{
	return connectionPool ? connectionPool->getAvailableConnections() : 0;
}

void Database::logPoolStatistics() const
{
	if (!connectionPool) {
		std::cout << "[Info - Database::logPoolStatistics] Connection pool not initialized" << std::endl;
		return;
	}

	auto snapshot = connectionPool->getSnapshot();

	std::cout << "[Info - Database::logPoolStatistics] Pool size: " << snapshot.poolSize
	          << ", Min: " << snapshot.minPoolSize
	          << ", Max: " << snapshot.maxPoolSize
	          << ", Active: " << snapshot.activeConnections
	          << ", Available: " << snapshot.availableConnections
	          << ", Total: " << snapshot.totalConnections << std::endl;
	std::cout << "[Info - Database::logPoolStatistics] Stats - Acquires: " << snapshot.totalAcquires
	          << ", Releases: " << snapshot.totalReleases
	          << ", Failed: " << snapshot.failedAcquires
	          << ", Reconnections: " << snapshot.reconnections
	          << ", Validation failures: " << snapshot.validationFailures << std::endl;
}

size_t Database::getActiveTransactionCount() const
{
	return TransactionManager::getInstance().getActiveTransactionCount();
}

int Database::getTransactionDepth()
{
	return TransactionManager::getInstance().getTransactionDepth();
}

void Database::cleanupThread()
{
	TransactionManager::getInstance().cleanupThread();
}

void Database::logTransactionStatistics() const
{
	TransactionManager::getInstance().logTransactionStats();
}

void Database::shutdown()
{
	if (connectionPool) {
		connectionPool->shutdown();
	}
}

DBResult::DBResult(MYSQL_RES* res)
{
	handle = res;

	size_t i = 0;

	MYSQL_FIELD* field = mysql_fetch_field(handle);
	while (field) {
		listNames[field->name] = i++;
		field = mysql_fetch_field(handle);
	}

	row = mysql_fetch_row(handle);
}

DBResult::~DBResult()
{
	mysql_free_result(handle);
}

std::string DBResult::getString(const std::string& s) const
{
	auto it = listNames.find(s);
	if (it == listNames.end()) {
		std::cout << "[Error - DBResult::getString] Column '" << s << "' does not exist in result set." << std::endl;
		return std::string();
	}

	if (row[it->second] == nullptr) {
		return std::string();
	}

	return std::string(row[it->second]);
}

const char* DBResult::getStream(const std::string& s, unsigned long& size) const
{
	auto it = listNames.find(s);
	if (it == listNames.end()) {
		std::cout << "[Error - DBResult::getStream] Column '" << s << "' doesn't exist in the result set" << std::endl;
		size = 0;
		return nullptr;
	}

	if (row[it->second] == nullptr) {
		size = 0;
		return nullptr;
	}

	size = mysql_fetch_lengths(handle)[it->second];
	return row[it->second];
}

bool DBResult::hasNext() const
{
	return row != nullptr;
}

bool DBResult::next()
{
	row = mysql_fetch_row(handle);
	return row != nullptr;
}

DBInsert::DBInsert(std::string query) : query(std::move(query))
{
	this->length = this->query.length();
}

bool DBInsert::addRow(const std::string& row)
{
	// adds new row to buffer
	const size_t rowLength = row.length();
	length += rowLength;
	if (length > Database::getInstance().getMaxPacketSize() && !execute()) {
		return false;
	}

	if (values.empty()) {
		values.reserve(rowLength + 2);
		values.push_back('(');
		values.append(row);
		values.push_back(')');
	} else {
		values.reserve(values.length() + rowLength + 3);
		values.push_back(',');
		values.push_back('(');
		values.append(row);
		values.push_back(')');
	}
	return true;
}

bool DBInsert::addRow(std::ostringstream& row)
{
	bool ret = addRow(row.str());
	row.str(std::string());
	return ret;
}

bool DBInsert::execute()
{
	if (values.empty()) {
		return true;
	}

	// executes buffer
	bool res = Database::getInstance().executeQuery(query + values);
	values.clear();
	length = query.length();
	return res;
}
