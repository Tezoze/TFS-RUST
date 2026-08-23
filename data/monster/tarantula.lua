-- Generated from XML. Source: monsters/tarantula.xml
return {
  schema = 1,
  name = "Tarantula",
  description = "a tarantula",
  race = "venom",
  experience = 120,
  speed = 67,
  mana_cost = 485,
  health = 225,
  max_health = 225,
  outfit = {
    look_type = 219,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4320,
  },
  change_target = { chance = 5 },
  target_strategy = { nearest = 70, weakest = 0, most_damage = 30, random = 0 },
  lose_target = { chance = 5 },
  flags = {
    hostile = true,
    summonable = true,
    illusionable = true,
    pushable = false,
    convinceable = true,
    can_push_items = true,
    can_push_creatures = false,
    target_distance = 1,
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 40,
      attack = 38,
      poison_cycles = 30,
      skill_factor = 2000,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "poisonfield",
      delay = 5,
      range = 1,
      shoot = "poison",
      effect = "poff",
    },
  },
  defenses = {
    armor = 20,
    defense = 20,
    spells = {
      {
        name = "speed",
        delay = 8,
        duration = 2000,
        speed = 95,
        speed_variation = 15,
        effect = "redshimmer",
      },
    },
  },
  immunities = {
    fire = false,
    energy = false,
    poison = true,
    physical = false,
    outfit = false,
    life_drain = false,
    paralyze = false,
    invisible = false,
  },
  loot = {
    { id = 2169, chance = 100 }, -- time ring
    { id = 2457, chance = 1000 }, -- steel helmet
    { id = 2510, chance = 2000 }, -- plate shield
    { id = 2148, chance = 30000, count_max = 30 }, -- gold coin
    { id = 2148, chance = 70000, count_max = 10 }, -- gold coin
    { id = 2478, chance = 3000 }, -- brass legs
  },
}
