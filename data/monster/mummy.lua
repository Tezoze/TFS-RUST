-- Generated from XML. Source: monsters/mummy.xml
return {
  schema = 1,
  name = "Mummy",
  description = "a mummy",
  race = "undead",
  experience = 150,
  speed = 35,
  mana_cost = 510,
  health = 240,
  max_health = 240,
  outfit = {
    look_type = 65,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2949,
  },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = true,
    pushable = false,
    convinceable = false,
    can_push_items = true,
    can_push_creatures = false,
    target_distance = 1,
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 50,
      attack = 32,
      poison_cycles = 65,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "speed",
      delay = 13,
      range = 7,
      duration = 10000,
      speed = -80,
      speed_variation = 40,
      effect = "redshimmer",
    },
    {
      name = "lifedrain",
      delay = 7,
      min = -30,
      max = -40,
      range = 1,
    },
  },
  defenses = {
    armor = 14,
    defense = 23,
  },
  immunities = {
    fire = false,
    energy = false,
    poison = true,
    physical = false,
    outfit = false,
    life_drain = true,
    paralyze = true,
    invisible = true,
  },
  voices = {
    { text = "I will ssswallow your sssoul!", yell = false },
    { text = "Ahkahra exura belil mort!", yell = false },
    { text = "Yohag Sssetham!", yell = false },
    { text = "I will tassste life again!", yell = false },
    { text = "Mort ulhegh dakh visss.", yell = false },
    { text = "Flesssh to dussst!", yell = false },
  },
  loot = {
    { id = 3976, chance = 70000, count_max = 3 }, -- worm
    { id = 2161, chance = 5000 }, -- strange talisman
    { id = 2134, chance = 4000 }, -- silver brooch
    { id = 2170, chance = 100 }, -- silver amulet
    { id = 2406, chance = 8000 }, -- short sword
    { id = 2411, chance = 2500 }, -- poison dagger
    { id = 2162, chance = 16000 }, -- magic light wand
    { id = 2148, chance = 40000, count_max = 80 }, -- gold coin
    { id = 2124, chance = 1500 }, -- crystal ring
    { id = 2529, chance = 200 }, -- black shield
    { id = 2144, chance = 1000 }, -- black pearl
  },
}
