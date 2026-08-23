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
    corpse = 2844,
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
    { id = 2187, chance = 1000 }, -- wand of inferno
    { id = 2509, chance = 15000 }, -- steel shield
    { id = 2457, chance = 3000 }, -- steel helmet
    { id = 2145, chance = 400 }, -- small diamond
    { id = 2406, chance = 25000 }, -- short sword
    { id = 2409, chance = 500 }, -- serpent sword
    { id = 2647, chance = 2000 }, -- plate legs
    { id = 2398, chance = 20000 }, -- mace
    { id = 2397, chance = 4000 }, -- longsword
    { id = 2177, chance = 100 }, -- life crystal
    { id = 2148, chance = 50000, count_max = 60 }, -- gold coin
    { id = 2148, chance = 80000, count_max = 45 }, -- gold coin
    { id = 2516, chance = 300 }, -- dragon shield
    { id = 2434, chance = 500 }, -- dragon hammer
    { id = 2672, chance = 45000, count_max = 3 }, -- dragon ham
    { id = 2387, chance = 1000 }, -- double axe
    { id = 2455, chance = 10000 }, -- crossbow
    { id = 2546, chance = 8000, count_max = 10 }, -- burst arrow
    { id = 2413, chance = 2000 }, -- broadsword
  },
}
