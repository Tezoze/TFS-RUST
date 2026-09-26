-- Generated from XML. Source: monsters/witch.xml
return {
  schema = 1,
  name = "Witch",
  description = "a witch",
  race = "blood",
  experience = 120,
  speed = 62,
  mana_cost = 0,
  health = 300,
  max_health = 300,
  outfit = {
    look_type = 54,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4247,
  },
  change_target = { chance = 50 },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  lose_target = { chance = 50 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = true,
    pushable = false,
    convinceable = false,
    can_push_items = true,
    can_push_creatures = false,
    target_distance = 4,
    run_health = 30,
  },
  attacks = {
    {
      name = "melee",
      skill = 18,
      attack = 18,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "firefield",
      delay = 8,
      range = 7,
      radius = 1,
      target = true,
      shoot = "fire",
    },
    {
      name = "fire",
      delay = 5,
      min = -25,
      max = -55,
      range = 7,
      shoot = "fire",
    },
  },
  defenses = {
    armor = 8,
    defense = 12,
  },
  immunities = {
    fire = false,
    energy = true,
    poison = true,
    physical = false,
    outfit = false,
    life_drain = false,
    paralyze = false,
    invisible = true,
  },
  voices = {
    { text = "Horax pokti!", yell = false },
    { text = "Hihihihi!", yell = false },
    { text = "Herba budinia ex!", yell = false },
  },
  loot = {
    { id = 3012, chance = 10000 }, -- wolf tooth chain
    { id = 3069, chance = 1000 }, -- volcanic rod
    { id = 3736, chance = 9000 }, -- star herb
    { id = 3290, chance = 500 }, -- silver dagger
    { id = 3293, chance = 40000 }, -- sickle
    { id = 3552, chance = 50000 }, -- leather boots
    { id = 3031, chance = 10000, count_max = 10 }, -- gold coin
    { id = 3083, chance = 2500 }, -- garlic necklace
    { id = 3598, chance = 30000, count_max = 8 }, -- cookie
    { id = 3562, chance = 20000 }, -- coat
    { id = 3607, chance = 40000 }, -- cheese
    { id = 3565, chance = 50000 }, -- cape
    { id = 3454, chance = 20000 }, -- broom
  },
}
