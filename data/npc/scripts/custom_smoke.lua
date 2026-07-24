-- NPC-7 smoke: custom predicate + custom action with read-after-write (Lua upvalue).

local marked = false

local npc = NpcType("CustomSmoke")
npc:appearance({ lookType = 136 })
npc:movement({ radius = 0, speed = 100, goStrength = 0 })
npc:health(100)

npc:onCustomPredicate("storage_ready", function(npcRef, player)
	return marked
end)

npc:onCustomAction("mark_and_greet", function(npcRef, player)
	marked = true
	-- Read-after-write in the same callback.
	if marked then
		npcRef:say("Marked.")
	else
		npcRef:say("Mark failed.")
	end
end)

npc:onCustomAction("boom", function(npcRef, player)
	error("intentional custom action error")
end)

npc:dialogue(NpcDialogue({
	policy = "queued_single_focus",
	rules = {
		{
			when = {
				{ situation = "address" },
				{ words = { "hi$" } },
				{ select = true },
			},
			actions = {
				{ say = "Say {mark} or {check}." },
			},
		},
		{
			when = {
				{ situation = "default" },
				{ words = { "mark$" } },
			},
			actions = {
				{ custom = "mark_and_greet" },
			},
		},
		{
			when = {
				{ situation = "default" },
				{ words = { "check$" } },
				{ custom = "storage_ready" },
			},
			actions = {
				{ say = "You are marked." },
			},
		},
		{
			when = {
				{ situation = "default" },
				{ words = { "check$" } },
			},
			actions = {
				{ say = "Not marked yet." },
			},
		},
		{
			when = {
				{ situation = "default" },
				{ words = { "boom$" } },
			},
			actions = {
				{ custom = "boom" },
				{ say = "Still here after boom." },
			},
		},
	},
}))

npc:register()
