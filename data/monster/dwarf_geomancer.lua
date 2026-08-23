-- Generated from XML. Source: monsters/dwarf geomancer.xml
return {
  schema = 1,
  name = "Dwarf Geomancer",
  description = "a dwarf geomancer",
  race = "blood",
  experience = 245,
  speed = 60,
  mana_cost = 0,
  health = 380,
  max_health = 380,
  outfit = {
    look_type = 66,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2987,
  },
  change_target = { chance = 50 },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  lose_target = { chance = 50 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = true,
    pushable = false,
    convinceable = false,
    can_push_items = true,
    can_push_creatures = true,
    target_distance = 4,
    run_health = 150,
  },
  attacks = {
    {
      name = "melee",
      skill = 50,
      attack = 30,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "manadrain",
      delay = 4,
      min = -50,
      max = -80,
      range = 7,
    },
    {
      name = "physical",
      delay = 3,
      min = -55,
      max = -105,
      range = 7,
      shoot = "largerock",
    },
  },
  defenses = {
    armor = 15,
    defense = 35,
    spells = {
      {
        name = "healing",
        delay = 2,
        min = 75,
        max = 125,
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
    paralyze = false,
    invisible = true,
  },
  voices = {
    { text = "Hail Durin!", yell = false },
    { text = "Earth is the strongest element.", yell = false },
    { text = "Dust to dust.", yell = false },
  },
  loot = {
    { id = 2787, chance = 60000, count_max = 2 }, -- white mushroom
    { id = 2468, chance = 20000 }, -- studded legs
    { id = 2175, chance = 400 }, -- spellbook
    { id = 2481, chance = 8000 }, -- soldier helmet
    { id = 2146, chance = 100 }, -- small sapphire
    { id = 2673, chance = 18000, count_max = 2 }, -- pear
    { id = 2162, chance = 12000 }, -- magic light wand
    { id = 2643, chance = 40000 }, -- leather boots
    { id = 2148, chance = 70000, count_max = 30 }, -- gold coin
    { id = 2213, chance = 300 }, -- dwarven ring
    { id = 2423, chance = 1000 }, -- clerical mace
    { id = 2260, chance = 10000 }, -- blank rune
    { id = 1987, chance = 50000 }, -- bag
  },
}
