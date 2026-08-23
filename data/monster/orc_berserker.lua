-- Generated from XML. Source: monsters/orc berserker.xml
return {
  schema = 1,
  name = "Orc Berserker",
  description = "an orc berserker",
  race = "blood",
  experience = 195,
  speed = 85,
  mana_cost = 590,
  health = 210,
  max_health = 210,
  outfit = {
    look_type = 8,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2864,
  },
  change_target = { chance = 10 },
  target_strategy = { nearest = 60, weakest = 40, most_damage = 0, random = 0 },
  lose_target = { chance = 10 },
  flags = {
    hostile = true,
    summonable = true,
    illusionable = true,
    pushable = false,
    convinceable = true,
    can_push_items = true,
    can_push_creatures = true,
    target_distance = 1,
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 50,
      attack = 65,
      skill_factor = 1200,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 12,
    defense = 12,
    spells = {
      {
        name = "speed",
        delay = 12,
        duration = 6000,
        speed = 50,
        speed_variation = 5,
        effect = "redshimmer",
      },
    },
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
    { text = "KRAK ORRRRRRK!", yell = true },
  },
  loot = {
    { id = 2044, chance = 8000 }, -- lamp
    { id = 2671, chance = 17000 }, -- ham
    { id = 2381, chance = 7000 }, -- halberd
    { id = 2148, chance = 55000, count_max = 12 }, -- gold coin
    { id = 2458, chance = 11000 }, -- chain helmet
    { id = 2464, chance = 10000 }, -- chain armor
    { id = 2378, chance = 6000 }, -- battle axe
  },
}
