-- Generated from XML. Source: monsters/centipede.xml
return {
  schema = 1,
  name = "Centipede",
  description = "a centipede",
  race = "blood",
  experience = 30,
  speed = 43,
  mana_cost = 335,
  health = 70,
  max_health = 70,
  outfit = {
    look_type = 124,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4289,
  },
  change_target = { chance = 6 },
  target_strategy = { nearest = 70, weakest = 0, most_damage = 30, random = 0 },
  lose_target = { chance = 6 },
  flags = {
    hostile = true,
    summonable = true,
    illusionable = true,
    pushable = true,
    convinceable = true,
    can_push_items = true,
    can_push_creatures = false,
    target_distance = 1,
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 30,
      attack = 24,
      poison_cycles = 22,
      skill_factor = 2000,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 8,
    defense = 9,
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
  loot = {
    { id = 2376, chance = 3000 }, -- sword
    { id = 2398, chance = 4500 }, -- mace
    { id = 2148, chance = 80000, count_max = 15 }, -- gold coin
  },
}
