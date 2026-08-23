-- Generated from XML. Source: monsters/rat.xml
return {
  schema = 1,
  name = "Rat",
  description = "a rat",
  race = "blood",
  experience = 5,
  speed = 27,
  mana_cost = 200,
  health = 20,
  max_health = 20,
  outfit = {
    look_type = 21,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2813,
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
    run_health = 5,
  },
  attacks = {
    {
      name = "melee",
      skill = 15,
      attack = 7,
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
    { id = 3976, chance = 50000, count_max = 3 }, -- worm
    { id = 2148, chance = 70000, count_max = 4 }, -- gold coin
    { id = 2696, chance = 40000 }, -- cheese
  },
}
