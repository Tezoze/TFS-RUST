-- Generated from XML. Source: monsters/orc rider.xml
return {
  schema = 1,
  name = "Orc Rider",
  description = "an orc rider",
  race = "blood",
  experience = 110,
  speed = 90,
  mana_cost = 490,
  health = 180,
  max_health = 180,
  outfit = {
    look_type = 4,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2972,
  },
  change_target = { chance = 5 },
  target_strategy = { nearest = 70, weakest = 30, most_damage = 0, random = 0 },
  lose_target = { chance = 5 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = false,
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
      skill = 48,
      attack = 41,
      skill_factor = 2000,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 9,
    defense = 20,
    spells = {
      {
        name = "speed",
        delay = 15,
        duration = 6000,
        speed = 45,
        speed_variation = 5,
        effect = "redshimmer",
      },
    },
  },
  immunities = {
    fire = false,
    energy = false,
    poison = false,
    physical = false,
    outfit = true,
    life_drain = false,
    paralyze = false,
    invisible = false,
  },
  voices = {
    { text = "Grrrrrrr", yell = false },
    { text = "Orc arga Huummmak!", yell = false },
  },
  loot = {
    { id = 2129, chance = 10000 }, -- wolf tooth chain
    { id = 2050, chance = 8000 }, -- torch
    { id = 2482, chance = 15000 }, -- studded helmet
    { id = 2483, chance = 600 }, -- scale armor
    { id = 2428, chance = 15000 }, -- orcish axe
    { id = 2425, chance = 1000 }, -- obsidian lance
    { id = 2666, chance = 30000, count_max = 3 }, -- meat
    { id = 2148, chance = 100, count_max = 80 }, -- gold coin
    { id = 2148, chance = 50000, count_max = 10 }, -- gold coin
    { id = 2513, chance = 1000 }, -- battle shield
    { id = 1988, chance = 30000 }, -- backpack
  },
}
