-- Generated from XML. Source: monsters/black sheep.xml
return {
  schema = 1,
  name = "Black Sheep",
  description = "a black sheep",
  race = "blood",
  experience = 0,
  speed = 18,
  mana_cost = 250,
  health = 20,
  max_health = 20,
  outfit = {
    look_type = 13,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2914,
  },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  flags = {
    hostile = false,
    summonable = true,
    illusionable = true,
    pushable = true,
    convinceable = true,
    can_push_items = false,
    can_push_creatures = false,
    target_distance = 1,
    run_health = 20,
  },
  attacks = {
    {
      name = "melee",
      skill = 0,
      attack = 0,
      skill_factor = 1200,
      skill_next_level = 0,
      skill_add_count = 0,
    },
  },
  defenses = {
    armor = 1,
    defense = 2,
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
    { text = "Maeh", yell = false },
  },
  loot = {
    { id = 2666, chance = 70000, count_max = 4 }, -- meat
  },
}
