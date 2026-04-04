local mType = Game.createMonsterType("Sea Serpent")
local monster = {}

monster.description = "a sea serpent"
monster.experience = 2300
monster.outfit = {
	lookType = 275,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8307
monster.health = 1950
monster.maxHealth = 1950
monster.race = "blood"
monster.speed = 480
monster.manaCost = 390
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
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	staticAttackChance = 70,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "CHHHRRRR", yell = false},
	{text = "HISSSS", yell = false},
}

monster.loot = {
	{id = 2148, chance = 98340, maxCount = 236}, -- gold coin
	{id = 2672, chance = 60450}, -- dragon ham
	{id = 2152, chance = 27080, maxCount = 3}, -- platinum coin
	{id = 10583, chance = 10080}, -- sea serpent scale
	{id = 2647, chance = 7130}, -- plate legs
	{id = 2146, chance = 5970, maxCount = 3}, -- small sapphire
	{id = 7588, chance = 5020}, -- strong health potion
	{id = 2409, chance = 4000}, -- serpent sword
	{id = 7589, chance = 3980}, -- strong mana potion
	{id = 8870, chance = 2950}, -- spirit cloak
	{id = 2214, chance = 1170}, -- ring of healing
	{id = 8911, chance = 1010}, -- northwind rod
	{id = 7590, chance = 910}, -- great mana potion
	{id = 7888, chance = 890}, -- glacier amulet
	{id = 7896, chance = 430}, -- glacier kilt
	{id = 2165, chance = 410}, -- stealth ring
	{id = 8871, chance = 400}, -- focus cape
	{id = 10220, chance = 110}, -- leviathan's amulet
	{id = 8878, chance = 90}, -- crystalline armor
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -250, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = -60, maxDamage = -300, effect = CONST_ME_SMALLPLANTS, target = false, length = 7, spread = 2, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -101, maxDamage = -300, effect = CONST_ME_ICEATTACK, target = false, length = 7, spread = 2, type = COMBAT_ICEDAMAGE},
	{name = "combat", interval = 2000, chance = 15, range = 5, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 45,
	armor = 25,
	{name = "combat", interval = 2000, chance = 30, minDamage = 70, maxDamage = 273, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 400, duration = 5000},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 30},
	{type = COMBAT_ENERGYDAMAGE, percent = -5},
	{type = COMBAT_PHYSICALDAMAGE, percent = -15},
	{type = COMBAT_DEATHDAMAGE, percent = 10},
}

monster.immunities = {
	{type = "ice", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "drown", combat = true, condition = true},
}


mType:register(monster)