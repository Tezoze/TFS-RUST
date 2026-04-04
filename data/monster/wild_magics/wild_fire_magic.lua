local mType = Game.createMonsterType("Wild Fire Magic")
local monster = {}

monster.description = "a wild fire magic"
monster.experience = 0
monster.health = 1
monster.maxHealth = 1
monster.race = "undead"
monster.speed = 210
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 10
}

monster.flags = {
	summonable = false,
	attackable = false,
	hostile = false,
	illusionable = false,
	convinceable = false,
	pushable = true,
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 7,
	staticAttackChance = 0,
	runHealth = 1,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.defenses = {
	defense = 0,
	armor = 0,
	{name = "combat", interval = 100, chance = 100, type = COMBAT_NONE},
}


mType:register(monster)