local mType = Game.createMonsterType("Undead Mine Worker")
local monster = {}

monster.description = "an undead mine worker"
monster.experience = 45
monster.outfit = {
	lookType = 33,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5972
monster.health = 65
monster.maxHealth = 65
monster.race = "undead"
monster.speed = 154
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 0
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = true,
	pushable = false,
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Ahrrr... uhmmm... hmm...", yell = false},
	{text = "Grrr...", yell = false},
	{text = "Urrrgh... gnarrr...", yell = false},
}

monster.loot = {
	{id = 2148, chance = 73000, maxCount = 10}, -- gold coin
	{id = 2230, chance = 42000}, -- bone
	{id = 2376, chance = 3850}, -- sword
	{id = 2398, chance = 26900}, -- mace
	{id = 2787, chance = 15400, maxCount = 3}, -- white mushroom
	{id = 2789, chance = 3850}, -- brown mushroom
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -30, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = -7, maxDamage = -13, range = 1, effect = CONST_ME_REDSHIMMER, target = false, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 15,
	armor = 5,
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
}


mType:register(monster)