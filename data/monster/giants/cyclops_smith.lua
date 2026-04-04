local mType = Game.createMonsterType("Cyclops Smith")
local monster = {}

monster.description = "a cyclops smith"
monster.experience = 255
monster.outfit = {
	lookType = 277,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7740
monster.health = 435
monster.maxHealth = 435
monster.race = "blood"
monster.speed = 204
monster.manaCost = 695
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 10
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = true,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Outis emoi g' onoma.", yell = false},
	{text = "Whack da humy!", yell = false},
	{text = "Ai humy phary ty kaynon", yell = false},
}

monster.loot = {
	{id = 2148, chance = 82920, maxCount = 70}, -- gold coin
	{id = 2207, chance = 90}, -- melee ring
	{id = 2378, chance = 5450}, -- battle axe
	{id = 2387, chance = 880}, -- double axe
	{id = 2417, chance = 5200}, -- battle hammer
	{id = 2442, chance = 2000}, -- heavy machete
	{id = 2490, chance = 200}, -- dark helmet
	{id = 2510, chance = 2000}, -- plate shield
	{id = 2513, chance = 6190}, -- battle shield
	{id = 2666, chance = 49950}, -- meat
	{id = 7398, chance = 140}, -- cyclops trophy
	{id = 7452, chance = 150}, -- spiked squelcher
	{id = 7588, chance = 390}, -- strong health potion
	{id = 10574, chance = 10280}, -- cyclops toe
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -150, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = 0, maxDamage = -70, range = 7, shootEffect = CONST_ANI_WHIRLWINDCLUB, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "drunk", interval = 2000, chance = 10, shootEffect = CONST_ANI_WHIRLWINDCLUB, effect = CONST_ME_STUN, target = true, duration = 4000},
}

monster.defenses = {
	defense = 25,
	armor = 28,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_FIREDAMAGE, percent = 10},
	{type = COMBAT_HOLYDAMAGE, percent = 1},
	{type = COMBAT_EARTHDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}


mType:register(monster)