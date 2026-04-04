local mType = Game.createMonsterType("Blood Priest")
local monster = {}

monster.description = "a blood priest"
monster.experience = 900
monster.outfit = {
	lookType = 553,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 21262
monster.health = 820
monster.maxHealth = 820
monster.race = "blood"
monster.speed = 210
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
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "The Blood God is thirsty!", yell = false},
	{text = "Give your blood to the Dark God!", yell = false},
}

monster.loot = {
	{id = 2147, chance = 3190, maxCount = 2}, -- small ruby
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 80}, -- gold coin
	{id = 2195, chance = 170}, -- boots of haste
	{id = 2436, chance = 190}, -- skull staff
	{id = 2663, chance = 2780}, -- mystic turban
	{id = 5909, chance = 2750}, -- white piece of cloth
	{id = 5911, chance = 640}, -- red piece of cloth
	{id = 7589, chance = 5940}, -- strong mana potion
	{id = 8901, chance = 300}, -- spellbook of warding
	{id = 8902, chance = 230}, -- spellbook of mind control
	{id = 8910, chance = 300}, -- underworld rod
	{id = 11237, chance = 15210}, -- book of necromantic rituals
	{id = 2156, chance = 790}, -- red gem
	{id = 21242, chance = 14630}, -- lancet
	{id = 21243, chance = 9520}, -- horoscope
	{id = 21245, chance = 14120}, -- blood tincture in a vial
	{id = 21246, chance = 14620}, -- incantation notes
	{id = 21247, chance = 7310}, -- pieces of magic chalk
	{id = 7456, chance = 80}, -- noble axe
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -80, target = false, condition = {type = CONDITION_POISON, totalDamage = 100, interval = 4000}},
	{name = "combat", interval = 2000, chance = 20, minDamage = -60, maxDamage = -100, range = 7, shootEffect = CONST_ANI_DEATH, effect = CONST_ME_SMALLCLOUDS, target = true, type = COMBAT_DEATHDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -40, maxDamage = -60, radius = 4, effect = CONST_ME_MAGIC_RED, target = false, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 3000, chance = 10, minDamage = -80, maxDamage = -130, range = 1, length = 7, spread = 0, effect = CONST_ME_HITAREA, target = true, type = COMBAT_MANADRAIN},
	{name = "condition", type = CONDITION_BLEEDING, interval = 2000, chance = 5, minDamage = -160, maxDamage = -290, range = 1, radius = 1, target = true},
}

monster.defenses = {
	defense = 30,
	armor = 30,
	{name = "combat", interval = 2000, chance = 20, minDamage = 80, maxDamage = 120, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = -5},
	{type = COMBAT_ENERGYDAMAGE, percent = 5},
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_ICEDAMAGE, percent = 5},
	{type = COMBAT_HOLYDAMAGE, percent = -5},
	{type = COMBAT_DEATHDAMAGE, percent = 5},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}

mType:register(monster)
