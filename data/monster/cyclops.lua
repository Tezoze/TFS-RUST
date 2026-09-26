-- Generated from XML. Source: monsters/cyclops.xml
return {
  schema = 1,
  name = "Cyclops",
  description = "a cyclops",
  race = "blood",
  experience = 150,
  speed = 55,
  mana_cost = 490,
  health = 260,
  max_health = 260,
  outfit = {
    look_type = 22,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 3989,
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
      skill = 50,
      attack = 30,
      skill_factor = 2000,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 17,
    defense = 24,
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
    { text = "Il lorstok human!", yell = false },
    { text = "Toks utat.", yell = false },
    { text = "Human, uh whil dyh!", yell = false },
    { text = "Youh ah trak!", yell = false },
    { text = "Let da mashing begin!", yell = false },
  },
  loot = {
    { id = 3012, chance = 200 }, -- wolf tooth chain
    { id = 3294, chance = 8000 }, -- short sword
    { id = 3410, chance = 2000 }, -- plate shield
    { id = 3577, chance = 50000 }, -- meat
    { id = 3582, chance = 20000, count_max = 2 }, -- ham
    { id = 3269, chance = 700 }, -- halberd
    { id = 3031, chance = 40000, count_max = 20 }, -- gold coin
    { id = 3031, chance = 70000, count_max = 10 }, -- gold coin
    { id = 3384, chance = 200 }, -- dark helmet
    { id = 3093, chance = 100 }, -- club ring
    { id = 3413, chance = 1500 }, -- battle shield
  },
}
