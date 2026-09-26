-- Generated from XML. Source: monsters/dragon.xml
return {
  schema = 1,
  name = "Dragon",
  description = "a dragon",
  race = "blood",
  experience = 700,
  speed = 45,
  mana_cost = 0,
  health = 1000,
  max_health = 1000,
  outfit = {
    look_type = 34,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4025,
  },
  change_target = { chance = 5 },
  target_strategy = { nearest = 70, weakest = 10, most_damage = 10, random = 10 },
  lose_target = { chance = 5 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = true,
    pushable = false,
    convinceable = false,
    can_push_items = true,
    can_push_creatures = true,
    target_distance = 1,
    run_health = 300,
  },
  attacks = {
    {
      name = "melee",
      skill = 55,
      attack = 42,
      skill_factor = 1100,
      skill_next_level = 100,
      skill_add_count = 2,
    },
    {
      name = "fire",
      delay = 9,
      min = -100,
      max = -160,
      length = 8,
      spread = 3,
      effect = "firearea",
    },
    {
      name = "fire",
      delay = 7,
      min = -55,
      max = -105,
      range = 7,
      radius = 4,
      target = true,
      shoot = "fire",
      effect = "firearea",
    },
  },
  defenses = {
    armor = 25,
    defense = 38,
    spells = {
      {
        name = "healing",
        delay = 8,
        min = 34,
        max = 56,
        effect = "blueshimmer",
      },
    },
  },
  immunities = {
    fire = true,
    energy = false,
    poison = true,
    physical = false,
    outfit = false,
    life_drain = false,
    paralyze = true,
    invisible = true,
  },
  voices = {
    { text = "GROOAAARRR", yell = true },
    { text = "FCHHHHH", yell = true },
  },
  loot = {
    { id = 3071, chance = 1000 }, -- wand of inferno
    { id = 3409, chance = 15000 }, -- steel shield
    { id = 3351, chance = 3000 }, -- steel helmet
    { id = 3028, chance = 400 }, -- small diamond
    { id = 3294, chance = 25000 }, -- short sword
    { id = 3297, chance = 500 }, -- serpent sword
    { id = 3557, chance = 2000 }, -- plate legs
    { id = 3286, chance = 20000 }, -- mace
    { id = 3285, chance = 4000 }, -- longsword
    { id = 3061, chance = 100 }, -- life crystal
    { id = 3031, chance = 50000, count_max = 60 }, -- gold coin
    { id = 3031, chance = 80000, count_max = 45 }, -- gold coin
    { id = 3416, chance = 300 }, -- dragon shield
    { id = 3322, chance = 500 }, -- dragon hammer
    { id = 3583, chance = 45000, count_max = 3 }, -- dragon ham
    { id = 3275, chance = 1000 }, -- double axe
    { id = 3349, chance = 10000 }, -- crossbow
    { id = 3449, chance = 8000, count_max = 10 }, -- burst arrow
    { id = 3301, chance = 2000 }, -- broadsword
  },
}
