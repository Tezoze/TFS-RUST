-- Generated from XML. Source: monsters/hyaena.xml
return {
  schema = 1,
  name = "Hyaena",
  description = "a hyaena",
  race = "blood",
  experience = 20,
  speed = 58,
  mana_cost = 275,
  health = 60,
  max_health = 60,
  outfit = {
    look_type = 94,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 3019,
  },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  flags = {
    hostile = true,
    summonable = true,
    illusionable = true,
    pushable = true,
    convinceable = true,
    can_push_items = false,
    can_push_creatures = false,
    target_distance = 1,
    run_health = 30,
  },
  attacks = {
    {
      name = "melee",
      skill = 18,
      attack = 11,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 1,
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
  voices = {
    { text = "Grrrrrr", yell = false },
    { text = "Hou hou hou!", yell = false },
  },
  loot = {
    { id = 3976, chance = 50000, count_max = 3 }, -- worm
    { id = 2666, chance = 50000, count_max = 2 }, -- meat
  },
}
