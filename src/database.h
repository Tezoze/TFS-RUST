// Copyright 2022 The Forgotten Server Authors. All rights reserved.
// Use of this source code is governed by the GPL-2.0 License that can be found in the LICENSE file.

#ifndef FS_DATABASE_H_A484B0CDFDE542838F506DCE3D40C693
#define FS_DATABASE_H_A484B0CDFDE542838F506DCE3D40C693

#include <boost/lexical_cast.hpp>

#include <mysql/mysql.h>
#include <memory>
#include <thread>
#include <unordered_map>

class ConnectionPool;

class DBResult;
using DBResult_ptr = std::shared_ptr<DBResult>;

class Database
{
	public:
		Database();
		~Database();

		// non-copyable
		Database(const Database&) = delete;
		Database& operator=(const Database&) = delete;

		/**
		 * Singleton implementation.
		 *
		 * @return database connection handler singleton
		 */
		static Database& getInstance()
		{
			static Database instance;
			return instance;
		}

		/**
		 * Connects to the database
		 *
		 * @return true on successful connection, false on error
		 */
		bool connect();

		/**
		 * Attempts to reconnect to the database (single connection mode only)
		 *
		 * @return true on successful reconnection, false on error
		 */
		bool reconnect();

		/**
		 * Executes command.
		 *
		 * Executes query which doesn't generates results (eg. INSERT, UPDATE, DELETE...).
		 *
		 * @param query command
		 * @return true on success, false on error
		 */
		bool executeQuery(const std::string& query);

		/**
		 * Queries database.
		 *
		 * Executes query which generates results (mostly SELECT).
		 *
		 * @return results object (nullptr on error)
		 */
		DBResult_ptr storeQuery(const std::string& query);

		/**
		 * Escapes string for query.
		 *
		 * Prepares string to fit SQL queries including quoting it.
		 *
		 * @param s string to be escaped
		 * @return quoted string
		 */
		std::string escapeString(const std::string& s) const;

		/**
		 * Escapes binary stream for query.
		 *
		 * Prepares binary stream to fit SQL queries.
		 *
		 * @param s binary stream
		 * @param length stream length
		 * @return quoted string
		 */
		std::string escapeBlob(const char* s, uint32_t length) const;

		/**
		 * Retrieve id of last inserted row
		 *
		 * @return id on success, 0 if last query did not result on any rows with auto_increment keys
		 */
		uint64_t getLastInsertId() const;

		/**
		 * Get database engine version
		 *
		 * @return the database engine version
		 */
		static const char* getClientVersion() {
			return mysql_get_client_info();
		}

		uint64_t getMaxPacketSize() const {
			return maxPacketSize;
		}

		/**
		 * Get connection pool statistics
		 *
		 * @return pool size, active connections, available connections
		 */
	size_t getPoolSize() const;
	size_t getActiveConnections() const;
	size_t getAvailableConnections() const;
	void logPoolStatistics() const;

	/**
	 * Get transaction statistics
	 */
	size_t getActiveTransactionCount() const;
	void logTransactionStatistics() const;

	/**
	 * Get transaction depth for current thread
	 */
	int getTransactionDepth();

	/**
	 * Cleanup transaction state for current thread
	 */
	void cleanupThread();

		/**
		 * Shutdown the database connection pool
		 */
		void shutdown();

	private:
		/**
		 * Transaction related methods.
		 *
		 * Methods for starting, committing and rolling back transaction. Each of the returns boolean value.
		 *
		 * @return true on success, false on error
		 */
		bool beginTransaction();
		bool rollback();
		bool commit();


	std::unique_ptr<ConnectionPool> connectionPool;
	MYSQL* singleConnection = nullptr; // Used when connection pooling is disabled
	mutable uint64_t maxPacketSize = 1048576;
	bool useConnectionPool = false;

	friend class DBTransaction;
	friend class TransactionManager;
};

class DBResult
{
	public:
		explicit DBResult(MYSQL_RES* res);
		~DBResult();

		// non-copyable
		DBResult(const DBResult&) = delete;
		DBResult& operator=(const DBResult&) = delete;

		template<typename T>
		T getNumber(const std::string& s) const
		{
			auto it = listNames.find(s);
			if (it == listNames.end()) {
				std::cout << "[Error - DBResult::getNumber] Column '" << s << "' doesn't exist in the result set" << std::endl;
				return static_cast<T>(0);
			}

			if (row[it->second] == nullptr) {
				return static_cast<T>(0);
			}

			T data;
			try {
				data = boost::lexical_cast<T>(row[it->second]);
			} catch (boost::bad_lexical_cast&) {
				data = 0;
			}
			return data;
		}

		std::string getString(const std::string& s) const;
		const char* getStream(const std::string& s, unsigned long& size) const;

		bool hasNext() const;
		bool next();

	private:
		MYSQL_RES* handle;
		MYSQL_ROW row;

		std::map<std::string, size_t> listNames;

	friend class Database;
};

/**
 * INSERT statement.
 */
class DBInsert
{
	public:
		explicit DBInsert(std::string query);
		bool addRow(const std::string& row);
		bool addRow(std::ostringstream& row);
		bool execute();

	private:
		std::string query;
		std::string values;
		size_t length;
};

class DBTransaction
{
	public:
		constexpr DBTransaction() = default;

		~DBTransaction() noexcept {
			// Safe cleanup - never throw exceptions from destructor
			if (state == STATE_START) {
				try {
					Database::getInstance().rollback();
				} catch (const std::exception& e) {
					// Log the error but don't rethrow - destructors must not throw
					std::cout << "[Critical - DBTransaction::~DBTransaction] Exception during rollback: " << e.what() << std::endl;
				} catch (...) {
					// Catch any other exceptions
					std::cout << "[Critical - DBTransaction::~DBTransaction] Unknown exception during rollback" << std::endl;
				}
			}
		}

		// non-copyable
		DBTransaction(const DBTransaction&) = delete;
		DBTransaction& operator=(const DBTransaction&) = delete;

		bool begin() {
			state = STATE_START;
			return Database::getInstance().beginTransaction();
		}

		bool commit() {
			if (state != STATE_START) {
				return false;
			}

			state = STATE_COMMIT;
			return Database::getInstance().commit();
		}

	private:
		enum TransactionStates_t {
			STATE_NO_START,
			STATE_START,
			STATE_COMMIT,
		};

		TransactionStates_t state = STATE_NO_START;
};

#endif
