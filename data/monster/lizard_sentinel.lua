-- Generated from XML. Source: monsters/lizard sentinel.xml
return {
  schema = 1,
  name = "Lizard Sentinel",
  description = "a lizard sentinel",
  race = "blood",
  experience = 105,
  speed = 50,
  mana_cost = 560,
  health = 265,
  max_health = 265,
  outfit = {
    look_type = 114,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4259,
  },
  change_target = { chance = 50 },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  lose_target = { chance = 50 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = true,
    pushable = true,
    convinceable = true,
    can_push_items = true,
    can_push_creatures = false,
    target_distance = 4,
    run_health = 10,
  },
  attacks = {
    {
      name = "melee",
      skill = 35,
      attack = 26,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "physical",
      delay = 15,
      min = -40,
      max = -70,
      range = 7,
      shoot = "spear",
    },
  },
  defenses = {
    armor = 17,
    defense = 24,
  },
  immunities = {
    fire = false,
    energy = false,
    poison = true,
    physical = false,
    outfit = false,
    life_drain = false,
    paralyze = false,
    invisible = true,
  },
  voices = {
    { text = "Tssss!", yell = false },
  },
  loot = {
    { id = 2389, chance = 10000, count_max = 3 }, -- spear
    { id = 2145, chance = 100 }, -- small diamond
    { id = 3974, chance = 300 }, -- sentinel shield
    { id = 2483, chance = 8000 }, -- scale armor
    { id = 2425, chance = 1200 }, -- obsidian lance
    { id = 2381, chance = 500 }, -- halberd
    { id = 2148, chance = 80000, count_max = 15 }, -- gold coin
    { id = 2464, chance = 9000 }, -- chain armor
  },
}
