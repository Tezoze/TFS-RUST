// Copyright 2022 The Forgotten Server Authors. All rights reserved.
// Use of this source code is governed by the GPL-2.0 License that can be found in the LICENSE file.

#ifndef FS_DECAY_H_F5229673AD6B4A2BAA2D38E5618F77B3A44B4248517D472553552E68
#define FS_DECAY_H_F5229673AD6B4A2BAA2D38E5618F77B3A44B4248517D472553552E68

#include "item.h"

class Decay
{
	public:
		void startDecay(Item* item, int32_t duration);
		void stopDecay(Item* item, int64_t timestamp);

	private:
		void checkDecay();

		uint64_t eventId = 0;
		std::map<int64_t, std::vector<Item*>> decayMap;
};

extern Decay g_decay;

#endif
