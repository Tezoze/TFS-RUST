-- Migrated from archived KeywordHandler bank.lua (NPC-7, core balance path).
-- Covers balance / deposit all / withdraw via Player bank APIs.

local npc = NpcType("Banker")
npc:appearance({ lookType = 472, lookHead = 40, lookBody = 95, lookLegs = 114, lookFeet = 27 })
npc:movement({ radius = 0, speed = 100 })
npc:health(100)

npc:onCustomAction("say_balance", function(npcRef, player)
	local bal = player:getBankBalance()
	npcRef:say("Your account balance is " .. bal .. " gold.")
end)

npc:onCustomAction("deposit_all", function(npcRef, player)
	local money = player:getMoney()
	if money < 1 then
		npcRef:say("You do not have enough gold.")
		return
	end
	player:depositMoney(money)
	npcRef:say("Alright, we have added the amount of " .. money .. " gold to your balance.")
end)

npc:onCustomPredicate("has_money", function(npcRef, player)
	return player:getMoney() >= 1
end)

npc:onCustomAction("withdraw_amount", function(npcRef, player)
	local bal = player:getBankBalance()
	if bal < 1 then
		npcRef:say("There is not enough gold on your account.")
		return
	end
	local amount = math.min(bal, 100)
	player:withdrawMoney(amount)
	npcRef:say("Here you are, " .. amount .. " gold.")
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
				{ say = "Welcome to the bank, %N! I can help you with your {balance}, {deposit}, or {withdraw}." },
			},
		},
		{
			when = {
				{ situation = "default" },
				{ words = { "balance$", "account$" } },
			},
			actions = {
				{ custom = "say_balance" },
			},
		},
		{
			when = {
				{ situation = "default" },
				{ words = { "deposit$" } },
				{ custom = "has_money" },
			},
			actions = {
				{ say = "Would you like to deposit all your gold?" },
				{ set = { var = "topic", value = 1 } },
			},
		},
		{
			when = {
				{ situation = "default" },
				{ words = { "deposit$" } },
			},
			actions = {
				{ say = "You do not have enough gold." },
			},
		},
		{
			when = {
				{ situation = "default" },
				{ words = { "yes$" } },
				{ expr = { session = "topic" }, op = "=", rhs = 1 },
			},
			actions = {
				{ custom = "deposit_all" },
				{ set = { var = "topic", value = 0 } },
			},
		},
		{
			when = {
				{ situation = "default" },
				{ words = { "withdraw$" } },
			},
			actions = {
				{ say = "Please tell me how much gold you would like to withdraw." },
				{ set = { var = "topic", value = 2 } },
			},
		},
		{
			when = {
				{ situation = "default" },
				{ capture = 1 },
				{ expr = { session = "topic" }, op = "=", rhs = 2 },
			},
			actions = {
				{ custom = "withdraw_amount" },
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
