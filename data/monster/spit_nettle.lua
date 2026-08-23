-- Generated from XML. Source: monsters/spit nettle.xml
return {
  schema = 1,
  name = "Spit Nettle",
  description = "a spit nettle",
  race = "venom",
  experience = 20,
  speed = 0,
  mana_cost = 0,
  health = 150,
  max_health = 150,
  outfit = {
    look_type = 221,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4326,
  },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = false,
    pushable = false,
    convinceable = false,
    can_push_items = false,
    can_push_creatures = false,
    target_distance = 1,
    run_health = 150,
  },
  attacks = {
    {
      name = "melee",
      skill = 15,
      attack = 9,
      poison_cycles = 18,
      skill_factor = 1000,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "poisoncondition",
      delay = 4,
      range = 7,
      cycle = 30,
      min_cycle = 8,
      shoot = "poison",
      effect = "greenbubble",
    },
    {
      name = "poison",
      delay = 7,
      min = -17,
      max = -37,
      range = 7,
      shoot = "poison",
      effect = "greenbubble",
    },
  },
  defenses = {
    armor = 12,
    defense = 33,
    spells = {
      {
        name = "healing",
        delay = 8,
        min = 8,
        max = 16,
        effect = "blueshimmer",
      },
    },
  },
  immunities = {
    fire = false,
    energy = true,
    poison = true,
    physical = false,
    outfit = true,
    life_drain = true,
    paralyze = true,
    invisible = true,
  },
  loot = {
    { id = 2802, chance = 5000 }, -- sling herb
    { id = 2802, chance = 1000 }, -- sling herb
    { id = 2804, chance = 10000 }, -- shadow herb
    { id = 2747, chance = 1000 }, -- grave flower
    { id = 2148, chance = 10000, count_max = 5 }, -- gold coin
  },
}
