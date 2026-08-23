-- Generated from XML. Source: monsters/dworc fleshhunter.xml
return {
  schema = 1,
  name = "Dworc Fleshhunter",
  description = "a dworc fleshhunter",
  race = "blood",
  experience = 35,
  speed = 34,
  mana_cost = 300,
  health = 85,
  max_health = 85,
  outfit = {
    look_type = 215,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4307,
  },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = true,
    pushable = true,
    convinceable = true,
    can_push_items = false,
    can_push_creatures = false,
    target_distance = 1,
    run_health = 8,
  },
  attacks = {
    {
      name = "melee",
      skill = 25,
      attack = 17,
      poison_cycles = 20,
      skill_factor = 1200,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "physical",
      delay = 12,
      min = -5,
      max = -15,
      range = 7,
      shoot = "throwingknife",
    },
  },
  defenses = {
    armor = 3,
    defense = 8,
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
  voices = {
    { text = "Grak brrretz!", yell = false },
    { text = "Grow truk grrrrr.", yell = false },
    { text = "Prek tars, dekklep zurk.", yell = false },
  },
  loot = {
    { id = 3967, chance = 500 }, -- tribal mask
    { id = 2050, chance = 5500 }, -- torch
    { id = 2229, chance = 3000, count_max = 3 }, -- skull
    { id = 3964, chance = 100 }, -- ripper lance
    { id = 2411, chance = 2000 }, -- poison dagger
    { id = 2467, chance = 11000 }, -- leather armor
    { id = 3965, chance = 500 }, -- hunting spear
    { id = 2148, chance = 80000, count_max = 10 }, -- gold coin
    { id = 2568, chance = 9000 }, -- cleaver
    { id = 2541, chance = 1000 }, -- bone shield
  },
}
