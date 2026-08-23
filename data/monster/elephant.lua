-- Generated from XML. Source: monsters/elephant.xml
return {
  schema = 1,
  name = "Elephant",
  description = "an elephant",
  race = "blood",
  experience = 160,
  speed = 55,
  mana_cost = 500,
  health = 320,
  max_health = 320,
  outfit = {
    look_type = 211,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4295,
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
      skill = 45,
      attack = 41,
      skill_factor = 2000,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 20,
    defense = 16,
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
    { text = "Hooooot-Toooooot!", yell = false },
    { text = "Tooooot.", yell = false },
    { text = "Troooooot!", yell = false },
  },
  loot = {
    { id = 3973, chance = 100 }, -- tusk shield
    { id = 2666, chance = 90000, count_max = 4 }, -- meat
    { id = 2671, chance = 60000, count_max = 3 }, -- ham
    { id = 3956, chance = 1000, count_max = 2 }, -- elephant tusk
  },
}
