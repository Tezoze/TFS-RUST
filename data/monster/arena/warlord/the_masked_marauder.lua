local mType = Game.createMonsterType("The Masked Marauder")
local monster = {}

monster.description = "The Masked Marauder"
monster.experience = 3500
monster.outfit = {
	lookType = 234,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7349
monster.health = 7320
monster.maxHealth = 7320
monster.race = "blood"
monster.speed = 250
monster.manaCost = 0
monster.maxSummons = 0

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	targetDistance = 1,
	canWalkOnEnergy = false,
	canWalkOnPoison = false,
	canWalkOnFire = true,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Didn't you leave your house door open?", yell = false},
	{text = "Oops, your shoelaces are open!", yell = false},
	{text = "Look! It's Ferumbras behind you!", yell = false},
	{text = "Stop! I give up!", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -640, target = false},
	{name = "combat", interval = 2000, chance = 40, minDamage = -38, maxDamage = -150, range = 7, shootEffect = CONST_ANI_THROWINGKNIFE, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 35,
	armor = 25,
	{name = "combat", interval = 1000, chance = 50, minDamage = 75, maxDamage = 125, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = -1},
	{type = COMBAT_ENERGYDAMAGE, percent = 30},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)