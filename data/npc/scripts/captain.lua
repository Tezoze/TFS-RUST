-- Migrated from archived KeywordHandler ship.lua (NPC-7).
-- Travel uses declarative dialogue + custom teleport actions.

local npc = NpcType("Captain")
npc:appearance({ lookType = 151, lookHead = 78, lookBody = 115, lookLegs = 85, lookFeet = 114 })
npc:movement({ radius = 0, speed = 100, goStrength = 0 })
npc:health(100)
npc:parameter("message_greet", "Welcome on board, |PLAYERNAME|.")

npc:onCustomAction("travel_trekolt", function(npcRef, player)
	player:teleportTo(Position(95, 117, 7))
end)

npc:onCustomAction("travel_rhyves", function(npcRef, player)
	player:teleportTo(Position(139, 337, 6))
end)

npc:onCustomAction("travel_varak", function(npcRef, player)
	player:teleportTo(Position(271, 516, 11))
end)

npc:onCustomAction("travel_saund", function(npcRef, player)
	player:teleportTo(Position(258, 602, 7))
end)

npc:dialogue(NpcDialogue({
	policy = "queued_single_focus",
	rules = {
		{
			when = {
				{ situation = "address" },
				{ words = { "hi$", "hello$" } },
				{ select = true },
			},
			actions = {
				{ say = "Welcome on board, %N. Where do you want to go? {Trekolt}, {Rhyves}, {Varak} or {Saund}?" },
			},
		},
		{
			when = {
				{ situation = "default" },
				{ words = { "captain$" } },
			},
			actions = {
				{ say = "I am the captain of this sailing-ship." },
			},
		},
		{
			when = {
				{ situation = "default" },
				{ words = { "trip$", "passage$" } },
			},
			actions = {
				{ say = "Where do you want to go? To {Trekolt}, {Rhyves}, {Varak} or {Saund}?" },
			},
		},
		{
			when = {
				{ situation = "default" },
				{ words = { "trekolt$" } },
			},
			actions = {
				{ say = "Do you seek a passage to Trekolt for 100 gold?" },
				{ set = { var = "topic", value = 1 } },
				{ set = { var = "price", value = 100 } },
			},
		},
		{
			when = {
				{ situation = "default" },
				{ words = { "rhyves$" } },
			},
			actions = {
				{ say = "Do you seek a passage to Rhyves for 120 gold?" },
				{ set = { var = "topic", value = 2 } },
				{ set = { var = "price", value = 120 } },
			},
		},
		{
			when = {
				{ situation = "default" },
				{ words = { "varak$" } },
			},
			actions = {
				{ say = "Do you seek a passage to Varak for 150 gold?" },
				{ set = { var = "topic", value = 3 } },
				{ set = { var = "price", value = 150 } },
			},
		},
		{
			when = {
				{ situation = "default" },
				{ words = { "saund$" } },
			},
			actions = {
				{ say = "Do you seek a passage to Saund for 150 gold?" },
				{ set = { var = "topic", value = 4 } },
				{ set = { var = "price", value = 150 } },
			},
		},
		{
			when = {
				{ situation = "default" },
				{ words = { "yes$" } },
				{ expr = { session = "topic" }, op = "=", rhs = 1 },
			},
			actions = {
				{ deleteMoney = true },
				{ custom = "travel_trekolt" },
				{ say = "Set the sails!" },
				{ idle = true },
			},
		},
		{
			when = {
				{ situation = "default" },
				{ words = { "yes$" } },
				{ expr = { session = "topic" }, op = "=", rhs = 2 },
			},
			actions = {
				{ deleteMoney = true },
				{ custom = "travel_rhyves" },
				{ say = "Set the sails!" },
				{ idle = true },
			},
		},
		{
			when = {
				{ situation = "default" },
				{ words = { "yes$" } },
				{ expr = { session = "topic" }, op = "=", rhs = 3 },
			},
			actions = {
				{ deleteMoney = true },
				{ custom = "travel_varak" },
				{ say = "Set the sails!" },
				{ idle = true },
			},
		},
		{
			when = {
				{ situation = "default" },
				{ words = { "yes$" } },
				{ expr = { session = "topic" }, op = "=", rhs = 4 },
			},
			actions = {
				{ deleteMoney = true },
				{ custom = "travel_saund" },
				{ say = "Set the sails!" },
				{ idle = true },
			},
		},
		{
			when = {
				{ situation = "default" },
				{ words = { "no$" } },
			},
			actions = {
				{ say = "We would like to serve you some time." },
				{ set = { var = "topic", value = 0 } },
			},
		},
		{
			when = {
				{ situation = "default" },
				{ words = { "bye$", "farewell$" } },
				{ select = true },
			},
			actions = {
				{ say = "Good bye." },
				{ idle = true },
			},
		},
	},
}))

npc:register()
