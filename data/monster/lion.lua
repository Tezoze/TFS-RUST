-- Generated from XML. Source: monsters/lion.xml
return {
  schema = 1,
  name = "Lion",
  description = "a lion",
  race = "blood",
  experience = 30,
  speed = 55,
  mana_cost = 320,
  health = 80,
  max_health = 80,
  outfit = {
    look_type = 41,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2889,
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
    run_health = 10,
  },
  attacks = {
    {
      name = "melee",
      skill = 32,
      attack = 20,
      skill_factor = 1100,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 6,
    defense = 13,
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
    { text = "Groarrr!", yell = false },
  },
  loot = {
    { id = 2666, chance = 45000, count_max = 3 }, -- meat
    { id = 2671, chance = 20000, count_max = 2 }, -- ham
  },
}
