-- Generated from XML. Source: monsters/tiger.xml
return {
  schema = 1,
  name = "Tiger",
  description = "a tiger",
  race = "blood",
  experience = 40,
  speed = 60,
  mana_cost = 420,
  health = 75,
  max_health = 75,
  outfit = {
    look_type = 125,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4292,
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
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 42,
      attack = 15,
      skill_factor = 2000,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 5,
    defense = 15,
    spells = {
      {
        name = "speed",
        delay = 9,
        duration = 3000,
        speed = 85,
        speed_variation = 15,
        effect = "redshimmer",
      },
    },
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
    { id = 2666, chance = 55000, count_max = 3 }, -- meat
    { id = 2671, chance = 22000, count_max = 2 }, -- ham
  },
}
