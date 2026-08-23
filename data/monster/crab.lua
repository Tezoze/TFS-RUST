-- Generated from XML. Source: monsters/crab.xml
return {
  schema = 1,
  name = "Crab",
  description = "a crab",
  race = "blood",
  experience = 30,
  speed = 60,
  mana_cost = 305,
  health = 55,
  max_health = 55,
  outfit = {
    look_type = 112,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4253,
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
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 25,
      attack = 15,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 28,
    defense = 30,
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
    { id = 2148, chance = 80000, count_max = 10 }, -- gold coin
    { id = 2667, chance = 20000 }, -- fish
  },
}
