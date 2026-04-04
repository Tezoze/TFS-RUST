// Copyright 2022 The Forgotten Server Authors. All rights reserved.
// Use of this source code is governed by the GPL-2.0 License that can be found in the LICENSE file.

#ifndef FS_NPC_H_B090D0CB549D4435AFA03647195D156F
#define FS_NPC_H_B090D0CB549D4435AFA03647195D156F

#include "creature.h"
#include "luascript.h"

#include <set>

class Npc;
class Player;
class NpcType;

// NPC event types for Lua callbacks
enum NpcsEvent_t {
	NPCS_EVENT_NONE,
	NPCS_EVENT_THINK,
	NPCS_EVENT_APPEAR,
	NPCS_EVENT_DISAPPEAR,
	NPCS_EVENT_MOVE,
	NPCS_EVENT_SAY,
	NPCS_EVENT_CLOSECHANNEL,
	NPCS_EVENT_BUYITEM,
	NPCS_EVENT_SELLITEM,
	NPCS_EVENT_CHECKITEM,
};

// Voice block for NPC speech
struct npcVoiceBlock_t {
	std::string text;
	bool yellText = false;
};

// NPC shop item info
struct NpcShopInfo {
	uint16_t itemId = 0;
	int32_t subType = -1;
	uint32_t buyPrice = 0;
	uint32_t sellPrice = 0;
	std::string name;
};

// Shop block for NpcType shop items (matches Canary format)
struct ShopBlock {
	uint16_t itemId = 0;
	int32_t subType = -1;
	uint32_t buyPrice = 0;
	uint32_t sellPrice = 0;
	std::string name;
};

// NpcType - stores NPC template data for Lua-defined NPCs
class NpcType
{
	public:
		struct NpcInfo {
			LuaScriptInterface* scriptInterface = nullptr;

			Outfit_t outfit = {};
			LightInfo light = {};

			uint8_t speechBubble = SPEECHBUBBLE_NONE;

			uint16_t currency = ITEM_GOLD_COIN;

			uint32_t yellChance = 0;
			uint32_t yellSpeedTicks = 0;
			uint32_t baseSpeed = 100;
			uint32_t walkInterval = 2000;

			int32_t creatureAppearEvent = -1;
			int32_t creatureDisappearEvent = -1;
			int32_t creatureMoveEvent = -1;
			int32_t creatureSayEvent = -1;
			int32_t thinkEvent = -1;
			int32_t playerCloseChannelEvent = -1;
			int32_t playerBuyEvent = -1;
			int32_t playerSellEvent = -1;
			int32_t playerCheckItemEvent = -1;

			int32_t health = 100;
			int32_t healthMax = 100;
			int32_t walkRadius = 10;

			bool canPushItems = false;
			bool canPushCreatures = false;
			bool pushable = true;
			bool floorChange = false;
			bool attackable = false;
			bool ignoreHeight = false;

			std::vector<npcVoiceBlock_t> voiceVector;
			std::vector<std::string> scripts;
			std::vector<NpcShopInfo> shopItems;
			std::vector<ShopBlock> shopItemVector;

			NpcsEvent_t eventType = NPCS_EVENT_NONE;
		};

		NpcType() = default;
		explicit NpcType(const std::string& initName);

		// non-copyable
		NpcType(const NpcType&) = delete;
		NpcType& operator=(const NpcType&) = delete;

		bool loadCallback(LuaScriptInterface* scriptInterface);

		std::string name;
		std::string nameDescription;
		NpcInfo info;

		bool fromLua = false;
};

class Npcs
{
	public:
		static void reload();
		static NpcType* getNpcType(const std::string& name, bool create = false);

	private:
		static std::map<std::string, std::unique_ptr<NpcType>> npcTypes;
};

class NpcScriptInterface final : public LuaScriptInterface
{
	public:
		NpcScriptInterface();

		bool loadNpcLib(const std::string& file);

	private:
		void registerFunctions();

		static int luaActionSay(lua_State* L);
		static int luaActionMove(lua_State* L);
		static int luaActionMoveTo(lua_State* L);
		static int luaActionTurn(lua_State* L);
		static int luaActionFollow(lua_State* L);
		static int luagetDistanceTo(lua_State* L);
		static int luaSetNpcFocus(lua_State* L);
		static int luaGetNpcCid(lua_State* L);
		static int luaGetNpcParameter(lua_State* L);
		static int luaOpenShopWindow(lua_State* L);
		static int luaCloseShopWindow(lua_State* L);
		static int luaDoSellItem(lua_State* L);

		// metatable
		static int luaNpcGetParameter(lua_State* L);
		static int luaNpcSetFocus(lua_State* L);

		static int luaNpcOpenShopWindow(lua_State* L);
		static int luaNpcCloseShopWindow(lua_State* L);

	private:
		bool initState() override;
		bool closeState() override;

		bool libLoaded;
};

class NpcEventsHandler
{
	public:
		NpcEventsHandler(const std::string& file, Npc* npc);
		NpcEventsHandler(NpcType* npcType, Npc* npc);

		void onCreatureAppear(Creature* creature);
		void onCreatureDisappear(Creature* creature);
		void onCreatureMove(Creature* creature, const Position& oldPos, const Position& newPos);
		void onCreatureSay(Creature* creature, SpeakClasses, const std::string& text);
		void onPlayerTrade(Player* player, int32_t callback, uint16_t itemId, uint8_t count, uint8_t amount, bool ignore = false, bool inBackpacks = false);
		void onPlayerBuyItem(Player* player, uint16_t itemId, uint8_t count, uint8_t amount, bool ignore = false, bool inBackpacks = false);
		void onPlayerSellItem(Player* player, uint16_t itemId, uint8_t count, uint8_t amount, bool ignoreEquipped = false);
		void onPlayerCloseChannel(Player* player);
		void onPlayerEndTrade(Player* player);
		void onThink();

		bool isLoaded() const;

	private:
		Npc* npc;
		NpcScriptInterface* scriptInterface;
		LuaScriptInterface* npcTypeScriptInterface = nullptr;  // For NpcType events

		int32_t creatureAppearEvent = -1;
		int32_t creatureDisappearEvent = -1;
		int32_t creatureMoveEvent = -1;
		int32_t creatureSayEvent = -1;
		int32_t playerCloseChannelEvent = -1;
		int32_t playerEndTradeEvent = -1;
		int32_t playerBuyEvent = -1;
		int32_t playerSellEvent = -1;
		int32_t thinkEvent = -1;
		bool loaded = false;
};

class Npc final : public Creature
{
	public:
		~Npc();

		// non-copyable
		Npc(const Npc&) = delete;
		Npc& operator=(const Npc&) = delete;

		Npc* getNpc() override {
			return this;
		}
		const Npc* getNpc() const override {
			return this;
		}

		bool isPushable() const override {
			return pushable && walkTicks != 0;
		}

		void setID() override {
			if (id == 0) {
				id = npcAutoID++;
			}
		}

		void removeList() override;
		void addList() override;

		static Npc* createNpc(const std::string& name);

		bool canSee(const Position& pos) const override;

		bool load();
		void reload();

		const std::string& getName() const override {
			return name;
		}
		const std::string& getNameDescription() const override {
			return name;
		}

		CreatureType_t getType() const override {
			return CREATURETYPE_NPC;
		}

		uint8_t getSpeechBubble() const override {
			return speechBubble;
		}
		void setSpeechBubble(const uint8_t bubble) {
			speechBubble = bubble;
		}

		void doSay(const std::string& text);
		void doSayToPlayer(Player* player, const std::string& text);

		bool doMoveTo(const Position& pos, int32_t minTargetDist = 1, int32_t maxTargetDist = 1,
		              bool fullPathSearch = true, bool clearSight = true, int32_t maxSearchDist = 0);

		int32_t getMasterRadius() const {
			return masterRadius;
		}
		const Position& getMasterPos() const {
			return masterPos;
		}
		void setMasterPos(Position pos, int32_t radius = 1) {
			masterPos = pos;
			if (masterRadius == -1) {
				masterRadius = radius;
			}
		}

		void onPlayerCloseChannel(Player* player);
		void onPlayerTrade(Player* player, int32_t callback, uint16_t itemId, uint8_t count,
		                   uint8_t amount, bool ignore = false, bool inBackpacks = false);
		void onPlayerBuyItem(Player* player, uint16_t itemId, uint8_t count,
		                     uint8_t amount, bool ignore = false, bool inBackpacks = false);
		void onPlayerSellItem(Player* player, uint16_t itemId, uint8_t count,
		                      uint8_t amount, bool ignoreEquipped = false);
		void onPlayerEndTrade(Player* player, int32_t buyCallback, int32_t sellCallback);

		void turnToCreature(Creature* creature);
		void setCreatureFocus(Creature* creature);

		NpcScriptInterface* getScriptInterface();

		static uint32_t npcAutoID;

	private:
		explicit Npc(const std::string& name);

		void onCreatureAppear(Creature* creature, bool isLogin) override;
		void onRemoveCreature(Creature* creature, bool isLogout) override;
		void onCreatureMove(Creature* creature, const Tile* newTile, const Position& newPos,
		                            const Tile* oldTile, const Position& oldPos, bool teleport) override;

		void onCreatureSay(Creature* creature, SpeakClasses type, const std::string& text) override;
		void onThink(uint32_t interval) override;
		std::string getDescription(int32_t lookDistance) const override;

		bool isImmune(CombatType_t) const override {
			return !attackable;
		}
		bool isImmune(ConditionType_t) const override {
			return !attackable;
		}
		bool isAttackable() const override {
			return attackable;
		}
		bool getNextStep(Direction& dir, uint32_t& flags) override;

		void setIdle(const bool idle);

		bool canWalkTo(const Position& fromPos, Direction dir) const;
		bool getRandomStep(Direction& dir) const;

		void reset();
		bool loadFromXml();
		bool loadFromNpcType(NpcType* npcType);

		void addShopPlayer(Player* player);
		void removeShopPlayer(Player* player);
		void closeAllShopWindows();

		std::map<std::string, std::string> parameters;

		std::set<Player*> shopPlayerSet;
		std::set<Player*> spectators;

		std::string name;
		std::string typeName;  // Original name from spawn (used for NpcType lookup)
		std::string filename;

		NpcEventsHandler* npcEventHandler;
		NpcType* npcType = nullptr;  // For Lua-defined NPCs

		Position masterPos;

		uint32_t walkTicks;
		int32_t focusCreature;
		int32_t masterRadius;

		uint8_t speechBubble;

		bool floorChange;
		bool attackable;
		bool ignoreHeight;
		bool loaded;
		bool isIdle;
		bool pushable;

		static NpcScriptInterface* scriptInterface;

		friend class Npcs;
		friend class NpcScriptInterface;
		friend class LuaScriptInterface;
};

#endif
