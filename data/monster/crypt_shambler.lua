-- Generated from XML. Source: monsters/crypt shambler.xml
return {
  schema = 1,
  name = "Crypt Shambler",
  description = "a crypt shambler",
  race = "undead",
  experience = 195,
  speed = 30,
  mana_cost = 580,
  health = 330,
  max_health = 330,
  outfit = {
    look_type = 100,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 3028,
  },
  change_target = { chance = 4 },
  target_strategy = { nearest = 70, weakest = 0, most_damage = 30, random = 0 },
  lose_target = { chance = 4 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = true,
    pushable = false,
    convinceable = true,
    can_push_items = true,
    can_push_creatures = true,
    target_distance = 1,
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 60,
      attack = 39,
      skill_factor = 1100,
      skill_next_level = 100,
      skill_add_count = 2,
    },
    {
      name = "lifedrain",
      delay = 8,
      min = -25,
      max = -55,
      range = 1,
    },
  },
  defenses = {
    armor = 30,
    defense = 25,
  },
  immunities = {
    fire = false,
    energy = false,
    poison = true,
    physical = false,
    outfit = false,
    life_drain = true,
    paralyze = true,
    invisible = false,
  },
  voices = {
    { text = "Uhhhhhhh!", yell = false },
    { text = "Aaaaahhhh!", yell = false },
    { text = "Hoooohhh!", yell = false },
    { text = "Chhhhhhh!", yell = false },
  },
  loot = {
    { id = 3976, chance = 90000, count_max = 10 }, -- worm
    { id = 2377, chance = 2000 }, -- two handed sword
    { id = 2399, chance = 1000, count_max = 3 }, -- throwing star
    { id = 2145, chance = 500 }, -- small diamond
    { id = 2227, chance = 20000 }, -- rotten meat
    { id = 2459, chance = 2000 }, -- iron helmet
    { id = 2148, chance = 30000, count_max = 30 }, -- gold coin
    { id = 2148, chance = 40000, count_max = 25 }, -- gold coin
    { id = 2450, chance = 1000 }, -- bone sword
    { id = 2541, chance = 1000 }, -- bone shield
    { id = 2230, chance = 50000 }, -- bone
  },
}
