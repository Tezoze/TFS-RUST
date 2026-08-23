-- Generated from XML. Source: monsters/omruc.xml
return {
  schema = 1,
  name = "Omruc",
  description = "",
  race = "undead",
  experience = 2950,
  speed = 115,
  mana_cost = 0,
  health = 4300,
  max_health = 4300,
  outfit = {
    look_type = 90,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 3016,
  },
  change_target = { chance = 7 },
  target_strategy = { nearest = 70, weakest = 10, most_damage = 20, random = 0 },
  lose_target = { chance = 7 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = false,
    pushable = false,
    convinceable = false,
    can_push_items = true,
    can_push_creatures = true,
    target_distance = 4,
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 75,
      attack = 65,
      poison_cycles = 65,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "speed",
      delay = 4,
      range = 7,
      duration = 50000,
      speed = -100,
      speed_variation = 10,
      effect = "redshimmer",
    },
    {
      name = "physical",
      delay = 5,
      min = -200,
      max = -400,
      range = 7,
      shoot = "arrow",
      effect = "explosion",
    },
    {
      name = "fire",
      delay = 8,
      min = -50,
      max = -450,
      range = 7,
      shoot = "burstarrow",
      effect = "explosion",
    },
    {
      name = "poison",
      delay = 5,
      min = -200,
      max = -500,
      range = 7,
      shoot = "poisonarrow",
      effect = "greenbubble",
    },
    {
      name = "lifedrain",
      delay = 20,
      min = -50,
      max = -350,
      range = 1,
    },
  },
  defenses = {
    armor = 60,
    defense = 65,
    spells = {
      {
        name = "invisible",
        delay = 3,
        duration = 20000,
        effect = "blueshimmer",
      },
      {
        name = "healing",
        delay = 6,
        min = 100,
        max = 200,
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
    { text = "Psssst, I am over chhhere.", yell = false },
    { text = "Now chhhou shhhee me ... Now chhhou don't.", yell = false },
    { text = "Chhhhou are marked ashhh my prey.", yell = false },
    { text = "Catchhhh me if chhhou can.", yell = false },
    { text = "Bullshhheye.", yell = false },
    { text = "Die!", yell = false },
  },
  summons = {
    max = 4,
    { name = "Stalker", delay = 7, max = 4 },
  },
  loot = {
    { id = 2154, chance = 1000 }, -- yellow gem
    { id = 2165, chance = 5000 }, -- stealth ring
    { id = 2145, chance = 10000, count_max = 3 }, -- small diamond
    { id = 2674, chance = 80000, count_max = 2 }, -- red apple
    { id = 2547, chance = 10000, count_max = 5 }, -- power bolt
    { id = 2545, chance = 60000, count_max = 20 }, -- poison arrow
    { id = 2148, chance = 35000, count_max = 95 }, -- gold coin
    { id = 2148, chance = 50000, count_max = 85 }, -- gold coin
    { id = 2148, chance = 70000, count_max = 80 }, -- gold coin
    { id = 2352, chance = 100000 }, -- crystal arrow
    { id = 2546, chance = 40000, count_max = 15 }, -- burst arrow
    { id = 2195, chance = 100 }, -- boots of haste
    { id = 2544, chance = 20000, count_max = 25 }, -- arrow
  },
}
