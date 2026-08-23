-- Generated from XML. Source: monsters/crocodile.xml
return {
  schema = 1,
  name = "Crocodile",
  description = "a crocodile",
  race = "blood",
  experience = 40,
  speed = 38,
  mana_cost = 350,
  health = 105,
  max_health = 105,
  outfit = {
    look_type = 119,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4277,
  },
  change_target = { chance = 5 },
  target_strategy = { nearest = 70, weakest = 0, most_damage = 30, random = 0 },
  lose_target = { chance = 5 },
  flags = {
    hostile = true,
    summonable = true,
    illusionable = true,
    pushable = false,
    convinceable = true,
    can_push_items = true,
    can_push_creatures = true,
    target_distance = 1,
    run_health = 10,
  },
  attacks = {
    {
      name = "melee",
      skill = 21,
      attack = 32,
      skill_factor = 2000,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 13,
    defense = 5,
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
  loot = {
    { id = 2666, chance = 70000, count_max = 4 }, -- meat
    { id = 2649, chance = 8000 }, -- leather legs
    { id = 2461, chance = 8000 }, -- leather helmet
    { id = 2671, chance = 40000, count_max = 2 }, -- ham
    { id = 2148, chance = 50000, count_max = 10 }, -- gold coin
    { id = 3982, chance = 100 }, -- crocodile boots
  },
}
