-- Generated from XML. Source: monsters/dipthrah.xml
return {
  schema = 1,
  name = "Dipthrah",
  description = "",
  race = "undead",
  experience = 2900,
  speed = 120,
  mana_cost = 0,
  health = 4200,
  max_health = 4200,
  outfit = {
    look_type = 87,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 3034,
  },
  change_target = { chance = 5 },
  target_strategy = { nearest = 70, weakest = 0, most_damage = 30, random = 0 },
  lose_target = { chance = 5 },
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
      skill = 70,
      attack = 65,
      poison_cycles = 65,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "drunk",
      delay = 9,
      radius = 4,
      duration = 60000,
      drunkness = 120,
      target = false,
      effect = "bluebubble",
    },
    {
      name = "speed",
      delay = 7,
      range = 7,
      duration = 50000,
      speed = -90,
      speed_variation = 20,
      effect = "redshimmer",
    },
    {
      name = "manadrain",
      delay = 7,
      min = -200,
      max = -500,
      range = 7,
      effect = "redshimmer",
    },
    {
      name = "lifedrain",
      delay = 5,
      min = -400,
      max = -800,
      range = 1,
    },
  },
  defenses = {
    armor = 65,
    defense = 75,
    spells = {
      {
        name = "healing",
        delay = 4,
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
    physical = true,
    outfit = true,
    life_drain = true,
    paralyze = true,
    invisible = true,
  },
  voices = {
    { text = "Come closer to learn the final lesson.", yell = false },
    { text = "I sense the weakness of your akh.", yell = false },
    { text = "Mortality and fear are your fate and your doom.", yell = false },
    { text = "Undeath will shatter my shackles.", yell = false },
    { text = "You can't escape death forever.", yell = false },
    { text = "You don't need this magic anymore.", yell = false },
    { text = "Feel the powers of my mind.", yell = false },
  },
  summons = {
    max = 4,
    { name = "Priestess", delay = 7, max = 4 },
  },
  loot = {
    { id = 2146, chance = 10000, count_max = 3 }, -- small sapphire
    { id = 2436, chance = 500 }, -- skull staff
    { id = 2446, chance = 100 }, -- pharaoh sword
    { id = 2354, chance = 100000 }, -- ornamented ankh
    { id = 2178, chance = 1000 }, -- mind stone
    { id = 2148, chance = 35000, count_max = 95 }, -- gold coin
    { id = 2148, chance = 50000, count_max = 85 }, -- gold coin
    { id = 2148, chance = 70000, count_max = 80 }, -- gold coin
    { id = 2167, chance = 5000 }, -- energy ring
    { id = 2158, chance = 1000 }, -- blue gem
    { id = 2193, chance = 500 }, -- ankh
  },
}
