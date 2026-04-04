local mType = Game.createMonsterType("Undead Gladiator")
local monster = {}

monster.description = "an undead gladiator"
monster.experience = 800
monster.outfit = {
	lookType = 306,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 9823
monster.health = 1000
monster.maxHealth = 1000
monster.race = "undead"
monster.speed = 220
monster.manaCost = 0
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
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnPoison = false,
	canWalkOnFire = true,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Let's battle it out in a duel!", yell = false},
	{text = "Bring it!", yell = false},
	{text = "I'll fight here in eternity and beyond.", yell = false},
	{text = "I will not give up!", yell = false},
	{text = "Another foolish adventurer who tries to beat me.", yell = false},
}

monster.loot = {
	{id = 2148, chance = 44000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 50500, maxCount = 48}, -- gold coin
	{id = 2165, chance = 30}, -- stealth ring
	{id = 2200, chance = 2200}, -- protection amulet
	{id = 2377, chance = 1900}, -- two handed sword
	{id = 2399, chance = 15700, maxCount = 18}, -- throwing star
	{id = 2419, chance = 11280}, -- scimitar
	{id = 2430, chance = 280}, -- knight axe
	{id = 2463, chance = 1700}, -- plate armor
	{id = 2465, chance = 4700}, -- brass armor
	{id = 2478, chance = 5500}, -- brass legs
	{id = 2490, chance = 1460}, -- dark helmet
	{id = 2497, chance = 100}, -- crusader helmet
	{id = 2647, chance = 2444}, -- plate legs
	{id = 3965, chance = 4200}, -- hunting spear
	{id = 5885, chance = 210}, -- flask of warrior's sweat
	{id = 7618, chance = 350}, -- health potion
	{id = 8872, chance = 5000}, -- belted cape
	{id = 10573, chance = 5200}, -- broken gladiator shield
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -250, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = 0, maxDamage = -135, range = 7, shootEffect = CONST_ANI_WHIRLWINDSWORD, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 45,
	armor = 35,
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_BLUESHIMMER},
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = -10},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_FIREDAMAGE, percent = 80},
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)