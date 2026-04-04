local mType = Game.createMonsterType("Deathbringer")
local monster = {}

monster.description = "Deathbringer"
monster.experience = 5100
monster.outfit = {
	lookType = 231,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7349
monster.health = 8440
monster.maxHealth = 8440
monster.race = "undead"
monster.speed = 300
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
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "YOU FOOLS WILL PAY!", yell = true},
	{text = "YOU ALL WILL DIE!!", yell = true},
	{text = "DEATH, DESTRUCTION!", yell = true},
	{text = "I will eat your soul!", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -465, target = false},
	{name = "combat", interval = 1000, chance = 10, minDamage = -80, maxDamage = -120, range = 7, radius = 4, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_REDSPARK, target = true, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 3000, chance = 17, minDamage = -300, maxDamage = -450, effect = CONST_ME_FIREAREA, target = false, length = 8, spread = 3, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 12, minDamage = -300, maxDamage = -450, effect = CONST_ME_POISON, target = false, length = 8, spread = 3, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -80, maxDamage = -100, radius = 6, effect = CONST_ME_POFF, target = false, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 3000, chance = 25, minDamage = -80, maxDamage = -150, range = 7, effect = CONST_ME_REDSHIMMER, target = false, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 15,
	armor = 15,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 10},
	{type = COMBAT_HOLYDAMAGE, percent = -15},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "ice", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "outfit", combat = false, condition = true},
	{type = "drunk", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)