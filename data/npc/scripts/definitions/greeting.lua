-- NPC-1 smoke definition: Quentin greeting / farewell (hand-authored).
-- Mirrors tests/fixtures/npc/greeting_farewell.json dialogue shape.
-- NPC-2 importer will replace/expand data/npc/scripts/definitions/.

local npc = NpcType("Quentin")
npc:appearance({ lookType = 57 })
npc:movement({ radius = 4, speed = 10, goStrength = 10 })
npc:health(100)
npc:sex(1)
npc:race(1)

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
				{ say = "Welcome, adventurer %N! If you are new in Tibia, ask me for help." },
			},
		},
		{
			when = {
				{ situation = "default" },
				{ words = { "bye$" } },
			},
			actions = {
				{ say = "Good bye, %N!" },
				{ idle = true },
			},
		},
	},
}))

npc:register()
