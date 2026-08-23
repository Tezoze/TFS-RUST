-- Generated from XML. Source: monsters/panda.xml
return {
  schema = 1,
  name = "Panda",
  description = "a panda",
  race = "blood",
  experience = 23,
  speed = 38,
  mana_cost = 300,
  health = 80,
  max_health = 80,
  outfit = {
    look_type = 123,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4286,
  },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  flags = {
    hostile = true,
    summonable = true,
    illusionable = true,
    pushable = false,
    convinceable = true,
    can_push_items = false,
    can_push_creatures = false,
    target_distance = 1,
    run_health = 15,
  },
  attacks = {
    {
      name = "melee",
      skill = 14,
      attack = 15,
      skill_factor = 2000,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 8,
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
    { text = "Grrrr", yell = false },
    { text = "Groar", yell = false },
  },
  loot = {
    { id = 2666, chance = 70000, count_max = 4 }, -- meat
    { id = 2671, chance = 40000, count_max = 2 }, -- ham
  },
}
