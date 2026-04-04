local mType = Game.createMonsterType("Primitive")
local monster = {}

monster.description = "Primitive"
monster.experience = 45
monster.outfit = {
	lookType = 143,
	lookHead = 1,
	lookBody = 1,
	lookLegs = 1,
	lookFeet = 1,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 200
monster.maxHealth = 200
monster.race = "blood"
monster.speed = 300
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 8
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 5,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "We don't need a future!", yell = false},
	{text = "I'll rook you all!", yell = false},
	{text = "They thought they'd beaten us!", yell = false},
	{text = "You are history!", yell = false},
	{text = "There can only be one world!", yell = false},
	{text = "Valor who?", yell = false},
	{text = "Die noob!", yell = false},
	{text = "There are no dragons!", yell = false},
	{text = "I'll quit forever! Again ...", yell = false},
	{text = "You all are noobs!", yell = false},
	{text = "Beware of the cyclops!", yell = false},
	{text = "Just had a disconnect.", yell = false},
	{text = "Magic is only good for girls!", yell = false},
	{text = "We'll be back!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 12500, maxCount = 10}, -- gold coin
	{id = 2385, chance = 10250}, -- sabre
	{id = 2386, chance = 12250}, -- axe
	{id = 2482, chance = 9500}, -- studded helmet
	{id = 2484, chance = 7000}, -- studded armor
	{id = 2526, chance = 1200}, -- studded shield
	{id = 6570, chance = 500},
	{id = 6571, chance = 500},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -32, target = false},
	{name = "combat", interval = 1000, chance = 20, range = 7, shootEffect = CONST_ANI_ENERGY, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 1000, chance = 34, minDamage = -20, maxDamage = -20, range = 7, radius = 3, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_FIREDAMAGE},
	{name = "outfit", interval = 1000, chance = 2, radius = 4, effect = CONST_ME_BLUEBUBBLE, target = false},
}

monster.defenses = {
	defense = 25,
	armor = 20,
}

monster.immunities = {
	{type = "energy", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "drunk", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)