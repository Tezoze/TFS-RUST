local mType = Game.createMonsterType("Vampire Bride")
local monster = {}

monster.description = "a vampire bride"
monster.experience = 1050
monster.outfit = {
	lookType = 312,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 9660
monster.health = 1200
monster.maxHealth = 1200
monster.race = "blood"
monster.speed = 200
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
	staticAttackChance = 80,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Kneel before your Mistress!", yell = false},
	{text = "Dead is the new alive.", yell = false},
	{text = "Come, let me kiss you, darling. Oh wait, I meant kill.", yell = false},
	{text = "Enjoy the pain - I know you love it.", yell = false},
	{text = "Are you suffering nicely enough?", yell = false},
	{text = "You won't regret you came to me, sweetheart.", yell = false},
}

monster.loot = {
	{id = 2127, chance = 1100}, -- emerald bangle
	{id = 2145, chance = 1020, maxCount = 2}, -- small diamond
	{id = 2148, chance = 90000, maxCount = 149}, -- gold coin
	{id = 2152, chance = 9910}, -- platinum coin
	{id = 2186, chance = 5500}, -- moonlight rod
	{id = 2195, chance = 220}, -- boots of haste
	{id = 7588, chance = 5000}, -- strong health potion
	{id = 7589, chance = 10210}, -- strong mana potion
	{id = 7733, chance = 200}, -- flower bouquet
	{id = 8873, chance = 1030}, -- hibiscus dress
	{id = 9447, chance = 60}, -- blood goblet
	{id = 9809, chance = 1010},
	{id = 9837, chance = 970}, -- velvet tapestry
	{id = 10602, chance = 10000}, -- vampire teeth
	{id = 12405, chance = 4950}, -- blood preservation
	{id = 13293, chance = 200}, -- leather whip
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -190, target = false},
	{name = "combat", interval = 3000, chance = 15, minDamage = -60, maxDamage = -130, range = 1, effect = CONST_ME_REDSHIMMER, target = true, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 10, minDamage = -60, maxDamage = -150, range = 7, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_MORTAREA, target = true, type = COMBAT_DEATHDAMAGE},
	{name = "combat", interval = 4000, chance = 5, minDamage = -60, maxDamage = -150, range = 7, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_HEARTS, target = true, type = COMBAT_DEATHDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -60, maxDamage = -150, range = 7, shootEffect = CONST_ANI_ENERGY, target = true, type = COMBAT_ENERGYDAMAGE},
}

monster.defenses = {
	defense = 20,
	armor = 55,
	{name = "combat", interval = 2000, chance = 15, minDamage = 40, maxDamage = 80, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 10},
	{type = COMBAT_EARTHDAMAGE, percent = 20},
	{type = COMBAT_DROWNDAMAGE, percent = 10},
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_HOLYDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)