// Copyright 2022 The Forgotten Server Authors. All rights reserved.
// Use of this source code is governed by the GPL-2.0 License that can be found in the LICENSE file.

#include "otpch.h"

#include "outputmessage.h"
#include "protocol.h"
#include "lockfree.h"
#include "scheduler.h"
#include "configmanager.h"

extern ConfigManager g_config;

extern Scheduler g_scheduler;

namespace {

const uint16_t OUTPUTMESSAGE_FREE_LIST_CAPACITY = 2048;
const std::chrono::milliseconds OUTPUTMESSAGE_AUTOSEND_DELAY {10};

void sendAll();

void scheduleSendAll()
{
	g_scheduler.addEvent(createSchedulerTask(OUTPUTMESSAGE_AUTOSEND_DELAY.count(), sendAll));
}

void sendAll()
{
	//dispatcher thread
	std::vector<Protocol_ptr> protocolsToSend = OutputMessagePool::getInstance().getBufferedProtocols();

	for (auto& protocol : protocolsToSend) {
		// Handle immediate sending
		auto& msg = protocol->getCurrentBuffer();
		if (msg) {
			protocol->send(std::move(msg));
		}
	}

	if (!protocolsToSend.empty()) {
		g_scheduler.addEvent(createSchedulerTask(OUTPUTMESSAGE_AUTOSEND_DELAY.count(), sendAll));
	}
}

}

void OutputMessagePool::addProtocolToAutosend(Protocol_ptr protocol)
{
	//dispatcher thread
	std::lock_guard<std::mutex> lock(bufferedProtocolsMutex);
	bool wasEmpty = bufferedProtocols.empty();
	bufferedProtocols.emplace_back(protocol);
	if (wasEmpty) {
		scheduleSendAll();
	}
}

void OutputMessagePool::removeProtocolFromAutosend(const Protocol_ptr& protocol)
{
	//dispatcher thread
	std::lock_guard<std::mutex> lock(bufferedProtocolsMutex);
	auto it = std::find(bufferedProtocols.begin(), bufferedProtocols.end(), protocol);
	if (it != bufferedProtocols.end()) {
		std::swap(*it, bufferedProtocols.back());
		bufferedProtocols.pop_back();
	}
}

OutputMessage_ptr OutputMessagePool::getOutputMessage()
{
	// LockfreePoolingAllocator<void,...> will leave (void* allocate) ill-formed because
	// of sizeof(T), so this guarantees that only one list will be initialized
	return std::allocate_shared<OutputMessage>(LockfreePoolingAllocator<void, OUTPUTMESSAGE_FREE_LIST_CAPACITY>());
}
