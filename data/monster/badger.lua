-- Generated from XML. Source: monsters/badger.xml
return {
  schema = 1,
  name = "Badger",
  description = "a badger",
  race = "blood",
  experience = 5,
  speed = 30,
  mana_cost = 200,
  health = 23,
  max_health = 23,
  outfit = {
    look_type = 105,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 3043,
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
    run_health = 10,
  },
  attacks = {
    {
      name = "melee",
      skill = 17,
      attack = 10,
      skill_factor = 1200,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 1,
    defense = 3,
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
    { id = 3976, chance = 10000 }, -- worm
    { id = 2666, chance = 40000 }, -- meat
  },
}
