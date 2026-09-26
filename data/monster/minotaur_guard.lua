-- Generated from XML. Source: monsters/minotaur guard.xml
return {
  schema = 1,
  name = "Minotaur Guard",
  description = "a minotaur guard",
  race = "blood",
  experience = 160,
  speed = 55,
  mana_cost = 550,
  health = 185,
  max_health = 185,
  outfit = {
    look_type = 29,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4057,
  },
  change_target = { chance = 5 },
  target_strategy = { nearest = 70, weakest = 10, most_damage = 20, random = 0 },
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
      skill = 50,
      attack = 35,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 15,
    defense = 32,
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
    { text = "Kirll Karrrl!", yell = false },
    { text = "Kaplar!", yell = false },
  },
  loot = {
    { id = 3577, chance = 10000 }, -- meat
    { id = 3559, chance = 15000 }, -- leather legs
    { id = 3276, chance = 10000 }, -- hatchet
    { id = 3031, chance = 60000, count_max = 20 }, -- gold coin
    { id = 3483, chance = 5000 }, -- fishing rod
    { id = 3275, chance = 400 }, -- double axe
    { id = 3558, chance = 1000 }, -- chain legs
    { id = 3358, chance = 3000 }, -- chain armor
    { id = 3359, chance = 4000 }, -- brass armor
    { id = 3413, chance = 2000 }, -- battle shield
  },
}
