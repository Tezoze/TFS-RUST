// Copyright 2022 The Forgotten Server Authors. All rights reserved.
// Use of this source code is governed by the GPL-2.0 License that can be found in the LICENSE file.

#include "otpch.h"

#include "protocol.h"
#include "outputmessage.h"
#include "rsa.h"
#include "xtea.h"
#include "configmanager.h"
#include <sstream>
#include <fstream>

extern RSA g_RSA;
extern ConfigManager g_config;

namespace {

void XTEA_encrypt(OutputMessage& msg, const xtea::round_keys& key)
{
	// The message must be a multiple of 8
	size_t paddingBytes = msg.getLength() % 8u;
	if (paddingBytes != 0) {
		msg.addPaddingBytes(8 - paddingBytes);
	}

	uint8_t* buffer = msg.getOutputBuffer();
	xtea::encrypt(buffer, msg.getLength(), key);
}

bool XTEA_decrypt(NetworkMessage& msg, const xtea::round_keys& key)
{
	if (((msg.getLength() - 6) & 7) != 0) {
		return false;
	}

	uint8_t* buffer = msg.getBuffer() + msg.getBufferPosition();
	xtea::decrypt(buffer, msg.getLength() - 6, key);

	uint16_t innerLength = msg.get<uint16_t>();
	if (innerLength + 8 > msg.getLength()) {
		return false;
	}

	msg.setLength(innerLength);
	return true;
}

}

void Protocol::onSendMessage(const OutputMessage_ptr& msg) const
{
	if (!rawMessages) {
		// Compress message if compression is enabled and message is large enough
		if (compressionEnabled && g_config.getBoolean(ConfigManager::PACKET_COMPRESSION_ENABLED) && msg->getLength() > 100) {
			int compressionLevel = g_config.getNumber(ConfigManager::PACKET_COMPRESSION_LEVEL);
			if (compressionLevel < 1) compressionLevel = 1;
			if (compressionLevel > 9) compressionLevel = 9;

			if (msg->compress(compressionLevel)) {
				if (g_config.getBoolean(ConfigManager::PACKET_COMPRESSION_DEBUG)) {
					std::cout << "[COMPRESSION] Message compressed: " << msg->getLength() << " bytes (level " << compressionLevel << ")" << std::endl;
				}
			}
		}

		msg->writeMessageLength();

		if (encryptionEnabled) {
			XTEA_encrypt(*msg, key);
			msg->addCryptoHeader(checksumEnabled);
		}
	}
}

void Protocol::onRecvMessage(NetworkMessage& msg)
{
	// Decrypt first (if enabled)
	if (encryptionEnabled && !XTEA_decrypt(msg, key)) {
		return;
	}

	// Then try to decompress (compression only happens for encrypted packets)
	if (encryptionEnabled && g_config.getBoolean(ConfigManager::PACKET_COMPRESSION_ENABLED)) {
		if (msg.decompress()) {
			if (g_config.getBoolean(ConfigManager::PACKET_COMPRESSION_DEBUG)) {
				std::cout << "[DECOMPRESSION] Message decompressed successfully" << std::endl;
			}
		}
		// Note: decompress() returns false for uncompressed packets, which is normal
	}

	parsePacket(msg);
}

OutputMessage_ptr Protocol::getOutputBuffer(int32_t size)
{
	//dispatcher thread
	if (!outputBuffer) {
		outputBuffer = OutputMessagePool::getOutputMessage();
	} else if ((outputBuffer->getLength() + size) > NetworkMessage::MAX_PROTOCOL_BODY_LENGTH) {
		send(outputBuffer);
		outputBuffer = OutputMessagePool::getOutputMessage();
	}
	return outputBuffer;
}

bool Protocol::RSA_decrypt(NetworkMessage& msg)
{
	if ((msg.getLength() - msg.getBufferPosition()) < 128) {
		return false;
	}

	g_RSA.decrypt(reinterpret_cast<char*>(msg.getBuffer()) + msg.getBufferPosition()); //does not break strict aliasing
	return msg.getByte() == 0;
}

uint32_t Protocol::getIP() const
{
	if (auto connection = getConnection()) {
		return connection->getIP();
	}

	return 0;
}

void Protocol::logPacket(const std::string& functionName, const char* file, int line) const
{
	currentFunctionName = functionName;
	currentFileName = file;
	currentLineNumber = line;
}

void Protocol::logPacketSend(const OutputMessage_ptr& msg) const
{
    // uint8_t* buffer = msg->getBuffer();
    // size_t len = msg->getLength();
    // std::cout << "[DEBUG] Sending login packet: opcode=";
    // if (len > 0) {
    //     std::cout << "0x" << std::hex << std::setw(2) << std::setfill('0') << static_cast<int>(buffer[0]);
    // } else {
    //     std::cout << "N/A";
    // }
    // std::cout << ", size=" << std::dec << len << ", first_bytes=";
    // for (size_t i = 0; i < std::min(len, size_t(6)); ++i) {
    //     std::cout << std::hex << std::setw(2) << std::setfill('0') << static_cast<int>(buffer[i]);
    //     if (i < 5) std::cout << " ";
    // }
    // std::cout << std::dec << std::endl;
}
