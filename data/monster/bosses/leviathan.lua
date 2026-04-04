local mType = Game.createMonsterType("Leviathan")
local monster = {}

monster.description = "Leviathan"
monster.experience = 5000
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
monster.health = 6000
monster.maxHealth = 6000
monster.race = "blood"
monster.speed = 758
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 2000,
	chance = 50
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
	targetDistance = 1,
	runHealth = 600,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 3000,
	chance = 20,
	{text = "CHHHRRRR", yell = false},
	{text = "HISSSS", yell = false},
}

monster.loot = {
	{id = 10220, chance = 100000}, -- leviathan's amulet
	{id = 10529, chance = 100000}, -- sea serpent trophy
	{id = 9809, chance = 83000},
	{id = 2152, chance = 82000, maxCount = 7}, -- platinum coin
	{id = 9812, chance = 77000},
	{id = 7428, chance = 58000}, -- bonebreaker
	{id = 2146, chance = 50000, maxCount = 5}, -- small sapphire
	{id = 7589, chance = 50000}, -- strong mana potion
	{id = 10521, chance = 14000}, -- moon backpack
	{id = 8887, chance = 1500}, -- frozen plate
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -500, target = false},
	{name = "combat", interval = 1000, chance = 8, minDamage = -130, maxDamage = -460, effect = CONST_ME_BIGPLANTS, target = false, length = 9, spread = 3, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 1000, chance = 10, minDamage = -365, maxDamage = -491, effect = CONST_ME_ICEAREA, target = false, length = 9, spread = 3, type = COMBAT_ICEDAMAGE},
	{name = "combat", interval = 1000, chance = 11, minDamage = -15, maxDamage = -20, radius = 4, effect = CONST_ME_BLUEBUBBLE, target = true, type = COMBAT_DROWNDAMAGE},
}

monster.defenses = {
	defense = 30,
	armor = 20,
	{name = "combat", interval = 2000, chance = 25, minDamage = 50, maxDamage = 350, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 30},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
	{type = COMBAT_PHYSICALDAMAGE, percent = -15},
	{type = COMBAT_DEATHDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "ice", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "drown", combat = true, condition = true},
}


mType:register(monster)