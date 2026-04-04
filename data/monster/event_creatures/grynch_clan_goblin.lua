local mType = Game.createMonsterType("Grynch Clan Goblin")
local monster = {}

monster.description = "Grynch Clan Goblin"
monster.experience = 4
monster.outfit = {
	lookType = 61,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6002
monster.health = 80
monster.maxHealth = 80
monster.race = "blood"
monster.speed = 200
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 0
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = false,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 11,
	staticAttackChance = 0,
	runHealth = 80,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "T'was not me hand in your pocket!", yell = false},
	{text = "Look! Cool stuff in house. Let's get it!", yell = false},
	{text = "Uhh! Nobody home! <chuckle>", yell = false},
	{text = "Me just borrowed it!", yell = false},
	{text = "Me no steal! Me found it!", yell = false},
	{text = "Me had it for five minutes. It's family heirloom now!", yell = false},
	{text = "Nice human won't hurt little, good goblin?", yell = false},
	{text = "Gimmegimme!", yell = false},
	{text = "Invite me in you lovely house plx!", yell = false},
	{text = "Other Goblin stole it!", yell = false},
	{text = "All presents mine!", yell = false},
	{text = "Me got ugly ones purse", yell = false},
	{text = "Free itans plz!", yell = false},
	{text = "Not me! Not me!", yell = false},
	{text = "Guys, help me here! Guys? Guys???", yell = false},
	{text = "That only much dust in me pocket! Honest!", yell = false},
	{text = "Can me have your stuff?", yell = false},
	{text = "Halp, Big dumb one is after me!", yell = false},
	{text = "Uh, So many shiny things!", yell = false},
	{text = "Utani hur hur hur!", yell = false},
	{text = "Mee? Stealing? Never!!!", yell = false},
	{text = "Oh what fun it is to steal a one-horse open sleigh!", yell = false},
	{text = "Must have it! Must have it!", yell = false},
	{text = "Where me put me lockpick?", yell = false},
	{text = "Catch me if you can!", yell = false},
}

monster.loot = {
	{id = 2072, chance = 5000}, -- lute
	{id = 2102, chance = 500}, -- flower bowl
	{id = 2111, chance = 7000, maxCount = 5}, -- snowball
	{id = 2114, chance = 1000}, -- piggy bank
	{id = 2148, chance = 22500, maxCount = 22}, -- gold coin
	{id = 2159, chance = 500, maxCount = 2}, -- scarab coin
	{id = 2163, chance = 4000}, -- magic light wand
	{id = 2260, chance = 5000}, -- blank rune
	{id = 2551, chance = 1500}, -- broom
	{id = 2560, chance = 1000}, -- mirror
	{id = 2661, chance = 4000}, -- scarf
	{id = 2674, chance = 700, maxCount = 3}, -- red apple
	{id = 2675, chance = 7000, maxCount = 3}, -- orange
	{id = 2679, chance = 7000, maxCount = 4}, -- cherry
	{id = 2687, chance = 7000, maxCount = 5}, -- cookie
	{id = 2688, chance = 5000, maxCount = 3}, -- candy cane
	{id = 2695, chance = 5000, maxCount = 2}, -- egg
	{id = 4873, chance = 4000}, -- explorer brooch
	{id = 5022, chance = 500, maxCount = 2}, -- orichalcum pearl
	{id = 5792, chance = 1000},
	{id = 5890, chance = 4000, maxCount = 5}, -- chicken feather
	{id = 5894, chance = 4000, maxCount = 3}, -- bat wing
	{id = 5902, chance = 4000}, -- honeycomb
	{id = 6277, chance = 7000, maxCount = 3}, -- lump of cake dough
	{id = 6393, chance = 1500}, -- valentine's cake
	{id = 6497, chance = 7000}, -- christmas present bag
	{id = 6501, chance = 4000, maxCount = 2}, -- gingerbreadman
	{id = 7909, chance = 3500, maxCount = 5}, -- walnut
	{id = 7910, chance = 3500, maxCount = 100}, -- peanut
}

monster.attacks = {
}

monster.defenses = {
	defense = 12,
	armor = 10,
	{name = "speed", interval = 1000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 500, duration = 5000},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)