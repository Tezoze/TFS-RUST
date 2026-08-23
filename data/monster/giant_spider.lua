-- Generated from XML. Source: monsters/giant spider.xml
return {
  schema = 1,
  name = "Giant Spider",
  description = "a giant spider",
  race = "venom",
  experience = 900,
  speed = 80,
  mana_cost = 0,
  health = 1300,
  max_health = 1300,
  outfit = {
    look_type = 38,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2857,
  },
  change_target = { chance = 10 },
  target_strategy = { nearest = 70, weakest = 20, most_damage = 0, random = 10 },
  lose_target = { chance = 10 },
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
      skill = 80,
      attack = 65,
      poison_cycles = 150,
      skill_factor = 1100,
      skill_next_level = 100,
      skill_add_count = 2,
    },
    {
      name = "poisonfield",
      delay = 6,
      range = 7,
      radius = 1,
      target = true,
      shoot = "poison",
    },
  },
  defenses = {
    armor = 30,
    defense = 40,
    spells = {
      {
        name = "speed",
        delay = 18,
        duration = 20000,
        speed = 65,
        speed_variation = 5,
        effect = "redshimmer",
      },
    },
  },
  immunities = {
    fire = true,
    energy = false,
    poison = true,
    physical = false,
    outfit = true,
    life_drain = true,
    paralyze = false,
    invisible = true,
  },
  summons = {
    max = 2,
    { name = "Poison Spider", delay = 10, max = 2 },
  },
  loot = {
    { id = 2169, chance = 700 }, -- time ring
    { id = 2457, chance = 5000 }, -- steel helmet
    { id = 2171, chance = 100 }, -- platinum amulet
    { id = 2463, chance = 10000 }, -- plate armor
    { id = 2477, chance = 300 }, -- knight legs
    { id = 2476, chance = 300 }, -- knight armor
    { id = 2148, chance = 99900, count_max = 11 }, -- gold coin
    { id = 2148, chance = 66600, count_max = 33 }, -- gold coin
    { id = 2148, chance = 33300, count_max = 55 }, -- gold coin
    { id = 2478, chance = 8000 }, -- brass legs
  },
}
