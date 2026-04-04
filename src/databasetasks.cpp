// Copyright 2022 The Forgotten Server Authors. All rights reserved.
// Use of this source code is governed by the GPL-2.0 License that can be found in the LICENSE file.

#include "otpch.h"

#include "databasetasks.h"
#include "tasks.h"

extern Dispatcher g_dispatcher;

void DatabaseTasks::start()
{
	db.connect();
	ThreadHolder::start();
}

void DatabaseTasks::threadMain()
{
	std::unique_lock<std::mutex> taskLockUnique(taskLock, std::defer_lock);
	while (getState() != THREAD_STATE_TERMINATED) {
		taskLockUnique.lock();
		if (tasks.empty()) {
			taskSignal.wait(taskLockUnique);
		}

		if (!tasks.empty()) {
			DatabaseTask task = std::move(tasks.front());
			tasks.pop_front();
			taskLockUnique.unlock();
			runTask(task);
		} else {
			taskLockUnique.unlock();
		}
	}
}

void DatabaseTasks::addTask(std::string query, std::function<void(DBResult_ptr, bool)> callback/* = nullptr*/, bool store/* = false*/)
{
	bool signal = false;
	taskLock.lock();
	if (getState() == THREAD_STATE_RUNNING) {
		signal = tasks.empty();
		tasks.emplace_back(std::move(query), std::move(callback), store);
	}
	taskLock.unlock();

	if (signal) {
		taskSignal.notify_one();
	}
}

void DatabaseTasks::runTask(const DatabaseTask& task)
{
	bool success;
	DBResult_ptr result;
	if (task.store) {
		result = db.storeQuery(task.query);
		success = (result != nullptr);
	} else {
		result = nullptr;
		success = db.executeQuery(task.query);
	}

	if (task.callback) {
		g_dispatcher.addTask(createTask(std::bind(task.callback, result, success)));
	}

	// [FIX] Safety check: Did this task leave a transaction open?
	if (db.getTransactionDepth() > 0) {
		std::cout << "[Warning] DatabaseTask left an open transaction! Forcing rollback." << std::endl;
		db.executeQuery("ROLLBACK");
		// Force cleanup of the thread state to be safe
		db.cleanupThread();
	}
}

void DatabaseTasks::flush()
{
	std::unique_lock<std::mutex> guard{ taskLock };
	auto timeout = std::chrono::steady_clock::now() + std::chrono::seconds(5); // 5 second timeout during shutdown

	while (!tasks.empty()) {
		auto task = std::move(tasks.front());
		tasks.pop_front();
		guard.unlock();

		// Check if we've exceeded the timeout
		if (std::chrono::steady_clock::now() > timeout) {
			std::cout << "[Warning - DatabaseTasks::flush] Timeout reached, skipping remaining tasks" << std::endl;
			return;
		}

		runTask(task);
		guard.lock();
	}
}

void DatabaseTasks::shutdown()
{
	taskLock.lock();
	setState(THREAD_STATE_TERMINATED);
	taskLock.unlock();

	// Flush remaining tasks with a short timeout to avoid hanging
	flush();

	// Now shutdown the database connection pool
	db.shutdown();

	taskSignal.notify_one();
}
