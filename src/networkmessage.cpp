// Copyright 2022 The Forgotten Server Authors. All rights reserved.
// Use of this source code is governed by the GPL-2.0 License that can be found in the LICENSE file.

#include "otpch.h"

#include "networkmessage.h"
#include "configmanager.h"

#include "container.h"
#include "creature.h"
#include "item.h"
#include <iomanip>

extern ConfigManager g_config;

std::string NetworkMessage::getString(uint16_t stringLen/* = 0*/)
{
	if (stringLen == 0) {
		stringLen = get<uint16_t>();
	}

	if (!canRead(stringLen)) {
		return std::string();
	}

	char* v = reinterpret_cast<char*>(buffer) + info.position; //does not break strict aliasing
	info.position += stringLen;
	return std::string(v, stringLen);
}

Position NetworkMessage::getPosition()
{
	Position pos;
	pos.x = get<uint16_t>();
	pos.y = get<uint16_t>();
	pos.z = getByte();
	return pos;
}

void NetworkMessage::addString(const std::string& value)
{
	size_t stringLen = value.length();
	if (!canAdd(stringLen + 2) || stringLen > 65535) {
		// Log if string is too large (for debugging)
		if (stringLen > 65535) {
			std::cout << "[NetworkMessage] Warning: String too large (" << stringLen << " bytes), max is 65535" << std::endl;
		}
		return;
	}

	add<uint16_t>(stringLen);
	memcpy(buffer + info.position, value.c_str(), stringLen);
	info.position += stringLen;
	info.length += stringLen;
}

void NetworkMessage::addDouble(double value, uint8_t precision/* = 2*/)
{
	addByte(precision);
	add<uint32_t>(static_cast<uint32_t>((value * std::pow(static_cast<float>(10), precision)) + std::numeric_limits<int32_t>::max()));
}

void NetworkMessage::addBytes(const char* bytes, size_t size)
{
	if (!canAdd(size) || size > 8192) {
		return;
	}

	memcpy(buffer + info.position, bytes, size);
	info.position += size;
	info.length += size;
}

void NetworkMessage::addPaddingBytes(size_t n)
{
	if (!canAdd(n)) {
		return;
	}

	memset(buffer + info.position, 0x33, n);
	info.length += n;
}

void NetworkMessage::addPosition(const Position& pos)
{
	add<uint16_t>(pos.x);
	add<uint16_t>(pos.y);
	addByte(pos.z);
}

void NetworkMessage::addItem(uint16_t id, uint8_t count, bool withDescription)
{
	const ItemType& it = Item::items[id];

	add<uint16_t>(it.clientId);

	addByte(0xFF); // MARK_UNMARKED

	if (it.stackable) {
		addByte(count);
	} else if (it.isSplash() || it.isFluidContainer()) {
		addByte(fluidMap[count & 7]);
	}

	if (it.isAnimation) {
		addByte(0xFE); // random phase (0xFF for async)
	}

	if (withDescription) {
		addString("");
	}

	// duration - no duration for template items
	addByte(0x00);
}

void NetworkMessage::addItem(const Item* item, bool withDescription)
{
	const ItemType& it = Item::items[item->getID()];

	add<uint16_t>(it.clientId);
	addByte(0xFF); // MARK_UNMARKED

	if (it.stackable) {
		addByte(std::min<uint16_t>(0xFF, item->getItemCount()));
	} else if (it.isSplash() || it.isFluidContainer()) {
		addByte(fluidMap[item->getFluidType() & 7]);
	}

	if (it.isAnimation) {
		addByte(0xFE); // random phase (0xFF for async)
	}

	if (withDescription) {
		addString(item->getDescription(0));
	}

	// duration - don't send for stackable items
	if (it.stackable) {
		addByte(0x00);
	} else if (item->hasAttribute(ITEM_ATTRIBUTE_DURATION) && item->getDuration() > 0) {
		if (item->isPickupable() && !item->getContainer()) {
			addByte(0x01);
			add<uint32_t>(item->getDuration());
			addByte(it.stopTime ? 1 : 0);
		} else {
			addByte(0x00);
		}
	} else {
		addByte(0x00);
	}
}

void NetworkMessage::addItemId(uint16_t itemId)
{
	add<uint16_t>(Item::items[itemId].clientId);
}

bool NetworkMessage::compress(int compressionLevel)
{
	if (info.length <= 0) {
		return false;
	}

	uint32_t originalSize = info.length;
	if (g_config.getBoolean(ConfigManager::PACKET_COMPRESSION_DEBUG)) {
		std::cout << "[COMPRESSION] Compressing message, original size: " << originalSize << " bytes" << std::endl;
	}

	// Allocate buffer for compressed data
	uLongf compressedSize = compressBound(info.length);
	std::vector<uint8_t> compressedBuffer(compressedSize);

	// Initialize z_stream for deflate
	z_stream zstream;
	zstream.zalloc = Z_NULL;
	zstream.zfree = Z_NULL;
	zstream.opaque = Z_NULL;
	zstream.next_in = buffer + INITIAL_BUFFER_POSITION;
	zstream.avail_in = info.length;
	zstream.next_out = compressedBuffer.data();
	zstream.avail_out = compressedSize;

	// Initialize deflate with raw deflate (no zlib header)
	int result = deflateInit2(&zstream, compressionLevel, Z_DEFLATED, -15, 8, Z_DEFAULT_STRATEGY);
	if (result != Z_OK) {
		return false;
	}

	// Compress the data
	result = deflate(&zstream, Z_FINISH);
	if (result != Z_STREAM_END) {
		deflateEnd(&zstream);
		return false;
	}

	compressedSize = zstream.total_out;
	deflateEnd(&zstream);

	// Check if compression actually reduced the size
	if (compressedSize >= static_cast<uLongf>(info.length)) {
		return false; // No benefit from compression
	}

	// Copy compressed data back to buffer, replacing the original data
	// Reserve space for compression header (1 byte: 0x01 = compressed)
	const size_t totalSize = compressedSize + 1;
	if (totalSize > MAX_BODY_LENGTH) {
		return false;
	}

	// Add compression header
	buffer[INITIAL_BUFFER_POSITION] = 0x01; // Compression flag

	// Copy compressed data after the header
	memcpy(buffer + INITIAL_BUFFER_POSITION + 1, compressedBuffer.data(), compressedSize);
	info.length = totalSize;
	info.position = INITIAL_BUFFER_POSITION + totalSize;

	if (g_config.getBoolean(ConfigManager::PACKET_COMPRESSION_DEBUG)) {
		std::cout << "[COMPRESSION] Compression successful: " << originalSize << " -> " << compressedSize + 1 << " bytes ("
		          << std::fixed << std::setprecision(1) << (static_cast<float>(compressedSize + 1) / originalSize * 100.0f) << "% of original)" << std::endl;
	}

	return true;
}

bool NetworkMessage::decompress()
{
	if (info.length <= 1) { // Need at least 1 byte for compression header
		return false;
	}

	// Check compression header
	uint8_t compressionFlag = buffer[INITIAL_BUFFER_POSITION];
	if (compressionFlag != 0x01) {
		// Not compressed, return false to indicate no decompression was needed
		return false;
	}

	uint32_t compressedSize = info.length - 1;
	if (g_config.getBoolean(ConfigManager::PACKET_COMPRESSION_DEBUG)) {
		std::cout << "[DECOMPRESSION] Decompressing message, compressed size: " << compressedSize << " bytes" << std::endl;
	}

	// Remove compression header from data to decompress
	const uint8_t* compressedData = buffer + INITIAL_BUFFER_POSITION + 1;

	// Allocate buffer for decompressed data (worst case)
	std::vector<uint8_t> decompressedBuffer(MAX_BODY_LENGTH);

	// Initialize z_stream for inflate
	z_stream zstream;
	zstream.zalloc = Z_NULL;
	zstream.zfree = Z_NULL;
	zstream.opaque = Z_NULL;
	zstream.next_in = const_cast<Bytef*>(compressedData);
	zstream.avail_in = compressedSize;
	zstream.next_out = decompressedBuffer.data();
	zstream.avail_out = MAX_BODY_LENGTH;

	// Initialize inflate with raw deflate (no zlib header)
	int result = inflateInit2(&zstream, -15);
	if (result != Z_OK) {
		return false;
	}

	// Decompress the data
	result = inflate(&zstream, Z_FINISH);
	if (result != Z_STREAM_END) {
		inflateEnd(&zstream);
		return false;
	}

	uint32_t decompressedSize = zstream.total_out;
	inflateEnd(&zstream);

	if (decompressedSize == 0 || decompressedSize > MAX_BODY_LENGTH) {
		return false;
	}

	// Copy decompressed data back to buffer
	memcpy(buffer + INITIAL_BUFFER_POSITION, decompressedBuffer.data(), decompressedSize);
	info.length = decompressedSize;
	info.position = INITIAL_BUFFER_POSITION;

	if (g_config.getBoolean(ConfigManager::PACKET_COMPRESSION_DEBUG)) {
		std::cout << "[DECOMPRESSION] Decompression successful: " << compressedSize << " -> " << decompressedSize << " bytes ("
		          << std::fixed << std::setprecision(1) << (static_cast<float>(decompressedSize) / compressedSize * 100.0f) << "% expansion)" << std::endl;
	}

	return true;
}
