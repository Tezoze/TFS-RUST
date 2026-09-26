-- Generated from XML. Source: monsters/black knight.xml
return {
  schema = 1,
  name = "Black Knight",
  description = "a black knight",
  race = "blood",
  experience = 1600,
  speed = 155,
  mana_cost = 0,
  health = 1800,
  max_health = 1800,
  outfit = {
    look_type = 131,
    look_head = 95,
    look_body = 95,
    look_legs = 95,
    look_feet = 95,
    corpse = 4240,
  },
  change_target = { chance = 3 },
  target_strategy = { nearest = 80, weakest = 10, most_damage = 10, random = 0 },
  lose_target = { chance = 3 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = false,
    pushable = false,
    convinceable = false,
    can_push_items = true,
    can_push_creatures = true,
    target_distance = 1,
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 88,
      attack = 60,
      skill_factor = 1200,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "physical",
      delay = 7,
      min = -145,
      max = -185,
      range = 7,
      shoot = "spear",
    },
  },
  defenses = {
    armor = 42,
    defense = 60,
  },
  immunities = {
    fire = true,
    energy = true,
    poison = true,
    physical = false,
    outfit = true,
    life_drain = false,
    paralyze = true,
    invisible = true,
  },
  voices = {
    { text = "MINE!", yell = true },
    { text = "NO PRISONERS!", yell = true },
    { text = "NO MERCY!", yell = true },
    { text = "By Bolg's Blood!", yell = false },
    { text = "You're no match for me!", yell = false },
  },
  loot = {
    { id = 3369, chance = 5000 }, -- warrior helmet
    { id = 3265, chance = 10000 }, -- two handed sword
    { id = 3351, chance = 10000 }, -- steel helmet
    { id = 3277, chance = 30000, count_max = 3 }, -- spear
    { id = 3016, chance = 800 }, -- ruby necklace
    { id = 3003, chance = 15000 }, -- rope
    { id = 3357, chance = 10000 }, -- plate armor
    { id = 3371, chance = 1000 }, -- knight legs
    { id = 3318, chance = 2500 }, -- knight axe
    { id = 3370, chance = 1000 }, -- knight armor
    { id = 3269, chance = 13000 }, -- halberd
    { id = 3031, chance = 22200, count_max = 90 }, -- gold coin
    { id = 3031, chance = 33300, count_max = 60 }, -- gold coin
    { id = 3302, chance = 300 }, -- dragon lance
    { id = 3384, chance = 2000 }, -- dark helmet
    { id = 3383, chance = 2000 }, -- dark armor
    { id = 3602, chance = 20000, count_max = 2 }, -- brown bread
    { id = 3372, chance = 13000 }, -- brass legs
    { id = 3079, chance = 500 }, -- boots of haste
    { id = 3305, chance = 7000 }, -- battle hammer
  },
}
