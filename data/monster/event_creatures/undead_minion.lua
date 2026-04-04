local mType = Game.createMonsterType("Undead Minion")
local monster = {}

monster.description = "Undead Minion"
monster.experience = 550
monster.outfit = {
	lookType = 37,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5963
monster.health = 850
monster.maxHealth = 850
monster.race = "undead"
monster.speed = 230
monster.manaCost = 620
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 1
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = true,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 40}, -- gold coin
	{id = 2260, chance = 10000}, -- blank rune
	{id = 2417, chance = 5000}, -- battle hammer
	{id = 2513, chance = 1000}, -- battle shield
	{id = 2515, chance = 5000}, -- guardian shield
	{id = 6570, chance = 5538},
	{id = 6571, chance = 1538},
}

monster.attacks = {
	{name = "melee", interval = 1000, minDamage = 0, maxDamage = -248, target = false},
	{name = "combat", interval = 1000, chance = 13, minDamage = -100, maxDamage = -160, radius = 4, effect = CONST_ME_MORTAREA, target = false, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 25,
	armor = 25,
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "drunk", combat = false, condition = true},
}


mType:register(monster)