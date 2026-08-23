-- Generated from XML. Source: monsters/dworc venomsniper.xml
return {
  schema = 1,
  name = "Dworc Venomsniper",
  description = "a dworc venomsniper",
  race = "blood",
  experience = 30,
  speed = 36,
  mana_cost = 300,
  health = 80,
  max_health = 80,
  outfit = {
    look_type = 216,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4310,
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
    target_distance = 4,
    run_health = 15,
  },
  attacks = {
    {
      name = "melee",
      skill = 20,
      attack = 10,
      skill_factor = 1200,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "poisoncondition",
      delay = 4,
      range = 5,
      cycle = 20,
      min_cycle = 6,
      shoot = "poison",
    },
  },
  defenses = {
    armor = 3,
    defense = 15,
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
    { id = 2410, chance = 8000, count_max = 2 }, -- throwing knife
    { id = 2229, chance = 1000, count_max = 2 }, -- skull
    { id = 2411, chance = 1500 }, -- poison dagger
    { id = 2545, chance = 5000, count_max = 3 }, -- poison arrow
    { id = 2467, chance = 10000 }, -- leather armor
    { id = 2148, chance = 80000, count_max = 10 }, -- gold coin
    { id = 2172, chance = 100 }, -- bronze amulet
    { id = 3983, chance = 100 }, -- bast skirt
  },
}
