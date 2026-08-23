-- Generated from XML. Source: monsters/beholder.xml
return {
  schema = 1,
  name = "Beholder",
  description = "a beholder",
  race = "blood",
  experience = 170,
  speed = 35,
  mana_cost = 0,
  health = 260,
  max_health = 260,
  outfit = {
    look_type = 17,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2908,
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
    can_push_creatures = false,
    target_distance = 4,
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 35,
      attack = 12,
      skill_factor = 1200,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "manadrain",
      delay = 20,
      min = -5,
      max = -25,
      range = 7,
      effect = "redshimmer",
    },
    {
      name = "lifedrain",
      delay = 17,
      min = -35,
      max = -45,
      range = 7,
      effect = "redshimmer",
    },
    {
      name = "poison",
      delay = 14,
      min = -5,
      max = -45,
      range = 7,
      shoot = "poison",
    },
    {
      name = "physical",
      delay = 13,
      min = -30,
      max = -50,
      range = 7,
      shoot = "death",
      effect = "mortarea",
    },
    {
      name = "fire",
      delay = 16,
      min = -25,
      max = -45,
      range = 7,
      shoot = "fire",
    },
    {
      name = "energy",
      delay = 15,
      min = -15,
      max = -45,
      range = 7,
      shoot = "energy",
    },
  },
  defenses = {
    armor = 5,
    defense = 20,
  },
  immunities = {
    fire = false,
    energy = false,
    poison = true,
    physical = false,
    outfit = false,
    life_drain = true,
    paralyze = false,
    invisible = true,
  },
  voices = {
    { text = "Eye for eye!", yell = false },
    { text = "Here's looking at you!", yell = false },
    { text = "Let me take a look at you!", yell = false },
    { text = "You've got the look!", yell = false },
  },
  summons = {
    max = 6,
    { name = "Skeleton", delay = 9, max = 6 },
  },
  loot = {
    { id = 2512, chance = 3000 }, -- wooden shield
    { id = 2377, chance = 4000 }, -- two handed sword
    { id = 2509, chance = 4000 }, -- steel shield
    { id = 2175, chance = 5000 }, -- spellbook
    { id = 2181, chance = 500 }, -- quagmire rod
    { id = 2394, chance = 7000 }, -- morning star
    { id = 2397, chance = 9000 }, -- longsword
    { id = 2148, chance = 70000, count_max = 20 }, -- gold coin
    { id = 2148, chance = 80000, count_max = 16 }, -- gold coin
    { id = 2148, chance = 90000, count_max = 12 }, -- gold coin
    { id = 2518, chance = 100 }, -- beholder shield
  },
}
