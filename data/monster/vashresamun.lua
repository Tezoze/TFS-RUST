-- Generated from XML. Source: monsters/vashresamun.xml
return {
  schema = 1,
  name = "Vashresamun",
  description = "",
  race = "undead",
  experience = 2950,
  speed = 115,
  mana_cost = 0,
  health = 4000,
  max_health = 4000,
  outfit = {
    look_type = 85,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 3016,
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
      skill = 60,
      attack = 45,
      poison_cycles = 65,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "lifedrain",
      delay = 5,
      min = -250,
      max = -550,
      radius = 5,
      target = false,
      effect = "rednote",
    },
    {
      name = "speed",
      delay = 8,
      range = 7,
      duration = 50000,
      speed = -90,
      speed_variation = 20,
      effect = "redshimmer",
    },
    {
      name = "lifedrain",
      delay = 15,
      min = -50,
      max = -750,
      range = 1,
    },
  },
  defenses = {
    armor = 40,
    defense = 75,
    spells = {
      {
        name = "healing",
        delay = 5,
        min = 250,
        max = 450,
        effect = "blueshimmer",
      },
    },
  },
  immunities = {
    fire = false,
    energy = false,
    poison = true,
    physical = false,
    outfit = true,
    life_drain = true,
    paralyze = true,
    invisible = true,
  },
  voices = {
    { text = "Heheheheee!", yell = false },
    { text = "Come my maidens, we have visitors!", yell = false },
    { text = "Are you enjoying my music?", yell = false },
    { text = "Dance a dance of death for me!", yell = false },
    { text = "If music is the food of death, drop dead.", yell = false },
    { text = "Chakka Chakka!", yell = false },
  },
  summons = {
    max = 2,
    { name = "Banshee", delay = 5, max = 2 },
  },
  loot = {
    { id = 2143, chance = 10000 }, -- white pearl
    { id = 2074, chance = 200 }, -- panpipes
    { id = 2072, chance = 10000 }, -- lute
    { id = 2148, chance = 35000, count_max = 95 }, -- gold coin
    { id = 2148, chance = 50000, count_max = 85 }, -- gold coin
    { id = 2148, chance = 70000, count_max = 80 }, -- gold coin
    { id = 2124, chance = 1500 }, -- crystal ring
    { id = 2445, chance = 100 }, -- crystal mace
    { id = 2656, chance = 1000 }, -- blue robe
    { id = 2349, chance = 100000 }, -- blue note
    { id = 2139, chance = 100 }, -- ancient tiara
  },
}
