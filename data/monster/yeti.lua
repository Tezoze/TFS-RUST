-- Generated from XML. Source: monsters/yeti.xml
return {
  schema = 1,
  name = "Yeti",
  description = "a yeti",
  race = "blood",
  experience = 460,
  speed = 85,
  mana_cost = 0,
  health = 950,
  max_health = 950,
  outfit = {
    look_type = 110,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 3055,
  },
  change_target = { chance = 5 },
  target_strategy = { nearest = 70, weakest = 0, most_damage = 30, random = 0 },
  lose_target = { chance = 5 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = false,
    pushable = false,
    convinceable = false,
    can_push_items = true,
    can_push_creatures = true,
    target_distance = 1,
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 79,
      attack = 53,
      skill_factor = 2000,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "energy",
      delay = 9,
      min = -120,
      max = -210,
      length = 3,
      spread = 3,
      effect = "poff",
    },
    {
      name = "physical",
      delay = 7,
      min = -170,
      max = -200,
      range = 7,
      shoot = "snowball",
      effect = "poff",
    },
  },
  defenses = {
    armor = 28,
    defense = 43,
  },
  immunities = {
    fire = false,
    energy = true,
    poison = false,
    physical = false,
    outfit = true,
    life_drain = false,
    paralyze = false,
    invisible = true,
  },
  voices = {
    { text = "Yooodelaaahooohooo!", yell = false },
    { text = "Yooodelaaaheeeheee!", yell = false },
  },
  loot = {
    { id = 2129, chance = 500 }, -- wolf tooth chain
    { id = 2111, chance = 50000, count_max = 22 }, -- snowball
    { id = 2666, chance = 75000, count_max = 4 }, -- meat
    { id = 2671, chance = 35000, count_max = 6 }, -- ham
    { id = 2148, chance = 30000, count_max = 20 }, -- gold coin
    { id = 2148, chance = 60000, count_max = 10 }, -- gold coin
    { id = 2644, chance = 100 }, -- bunnyslippers
  },
}
