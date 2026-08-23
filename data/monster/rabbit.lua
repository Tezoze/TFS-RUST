-- Generated from XML. Source: monsters/rabbit.xml
return {
  schema = 1,
  name = "Rabbit",
  description = "a rabbit",
  race = "blood",
  experience = 0,
  speed = 50,
  mana_cost = 220,
  health = 15,
  max_health = 15,
  outfit = {
    look_type = 74,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2992,
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
    run_health = 15,
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
  loot = {
    { id = 2666, chance = 85000, count_max = 2 }, -- meat
    { id = 2684, chance = 10000 }, -- carrot
  },
}
