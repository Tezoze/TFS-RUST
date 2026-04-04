local mType = Game.createMonsterType("Coldheart")
local monster = {}

monster.description = "Coldheart"
monster.experience = 3500
monster.outfit = {
	lookType = 261,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7282
monster.health = 7000
monster.maxHealth = 7000
monster.race = "undead"
monster.speed = 195
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 2000,
	chance = 9
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	staticAttackChance = 50,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 100, attack = 40, target = false},
	{name = "combat", interval = 2000, chance = 25, minDamage = 0, maxDamage = -710, effect = CONST_ME_ICEAREA, target = false, length = 8, spread = 0, type = COMBAT_ICEDAMAGE},
}

monster.defenses = {
	defense = 26,
	armor = 25,
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "ice", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
}


mType:register(monster)