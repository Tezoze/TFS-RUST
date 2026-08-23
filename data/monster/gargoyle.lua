-- Generated from XML. Source: monsters/gargoyle.xml
return {
  schema = 1,
  name = "Gargoyle",
  description = "a gargoyle",
  race = "undead",
  experience = 150,
  speed = 60,
  mana_cost = 0,
  health = 250,
  max_health = 250,
  outfit = {
    look_type = 95,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 3022,
  },
  change_target = { chance = 5 },
  target_strategy = { nearest = 70, weakest = 0, most_damage = 30, random = 0 },
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
    run_health = 30,
  },
  attacks = {
    {
      name = "melee",
      skill = 45,
      attack = 24,
      skill_factor = 2000,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 26,
    defense = 30,
    spells = {
      {
        name = "healing",
        delay = 10,
        min = 5,
        max = 15,
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
    life_drain = true,
    paralyze = false,
    invisible = false,
  },
  voices = {
    { text = "Harrrr Harrrr!", yell = false },
    { text = "Stone sweet stone.", yell = false },
    { text = "Feel my claws, softskin.", yell = false },
    { text = "Chhhhhrrrrk!", yell = false },
    { text = "There is a stone in your shoe!", yell = false },
  },
  loot = {
    { id = 2129, chance = 200 }, -- wolf tooth chain
    { id = 2448, chance = 8000 }, -- studded club
    { id = 2457, chance = 200 }, -- steel helmet
    { id = 1294, chance = 10000, count_max = 10 }, -- small stone
    { id = 2394, chance = 1000 }, -- morning star
    { id = 2666, chance = 50000 }, -- meat
    { id = 2671, chance = 20000, count_max = 2 }, -- ham
    { id = 2148, chance = 40000, count_max = 20 }, -- gold coin
    { id = 2148, chance = 70000, count_max = 10 }, -- gold coin
    { id = 2489, chance = 200 }, -- dark armor
    { id = 2209, chance = 100 }, -- club ring
    { id = 2513, chance = 1500 }, -- battle shield
  },
}
