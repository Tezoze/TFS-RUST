-- Generated from XML. Source: monsters/minotaur archer.xml
return {
  schema = 1,
  name = "Minotaur Archer",
  description = "a minotaur archer",
  race = "blood",
  experience = 65,
  speed = 40,
  mana_cost = 390,
  health = 100,
  max_health = 100,
  outfit = {
    look_type = 24,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4052,
  },
  change_target = { chance = 50 },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  lose_target = { chance = 50 },
  flags = {
    hostile = true,
    summonable = true,
    illusionable = true,
    pushable = false,
    convinceable = true,
    can_push_items = false,
    can_push_creatures = false,
    target_distance = 4,
    run_health = 10,
  },
  attacks = {
    {
      name = "melee",
      skill = 20,
      attack = 15,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "physical",
      delay = 3,
      min = -45,
      max = -85,
      range = 7,
      shoot = "bolt",
    },
  },
  defenses = {
    armor = 7,
    defense = 8,
  },
  immunities = {
    fire = false,
    energy = false,
    poison = false,
    physical = false,
    outfit = false,
    life_drain = false,
    paralyze = false,
    invisible = false,
  },
  voices = {
    { text = "Ruan Wihmpy!", yell = false },
    { text = "Kaplar!", yell = false },
  },
  loot = {
    { id = 3375, chance = 2000 }, -- soldier helmet
    { id = 3377, chance = 1000 }, -- scale armor
    { id = 3577, chance = 10000 }, -- meat
    { id = 3559, chance = 5000 }, -- leather legs
    { id = 3355, chance = 5000 }, -- leather helmet
    { id = 3031, chance = 15000, count_max = 20 }, -- gold coin
    { id = 3031, chance = 70000, count_max = 10 }, -- gold coin
    { id = 3349, chance = 10000 }, -- crossbow
    { id = 3359, chance = 2000 }, -- brass armor
    { id = 3446, chance = 80000, count_max = 5 }, -- bolt
    { id = 3446, chance = 50000, count_max = 15 }, -- bolt
  },
}
