-- Generated from XML. Source: monsters/fire devil.xml
return {
  schema = 1,
  name = "Fire Devil",
  description = "a fire devil",
  race = "blood",
  experience = 110,
  speed = 50,
  mana_cost = 530,
  health = 200,
  max_health = 200,
  outfit = {
    look_type = 40,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2886,
  },
  change_target = { chance = 50 },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  lose_target = { chance = 50 },
  flags = {
    hostile = true,
    summonable = true,
    illusionable = true,
    pushable = false,
    convinceable = true,
    can_push_items = true,
    can_push_creatures = false,
    target_distance = 4,
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 30,
      attack = 22,
      skill_factor = 1100,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "fire",
      delay = 4,
      min = -20,
      max = -50,
      range = 7,
      radius = 2,
      target = true,
      shoot = "fire",
      effect = "firearea",
    },
    {
      name = "fire",
      delay = 9,
      min = -60,
      max = -90,
      range = 7,
      radius = 4,
      target = true,
      shoot = "fire",
      effect = "firearea",
    },
  },
  defenses = {
    armor = 13,
    defense = 15,
  },
  immunities = {
    fire = true,
    energy = false,
    poison = false,
    physical = false,
    outfit = false,
    life_drain = false,
    paralyze = false,
    invisible = false,
  },
  voices = {
    { text = "Hot, eh?", yell = false },
    { text = "Hell, oh hell!", yell = false },
  },
  loot = {
    { id = 2185, chance = 500 }, -- volcanic rod
    { id = 2050, chance = 15000, count_max = 2 }, -- torch
    { id = 2150, chance = 300 }, -- small amethyst
    { id = 2419, chance = 6000 }, -- scimitar
    { id = 2548, chance = 50000 }, -- pitchfork
    { id = 2515, chance = 200 }, -- guardian shield
    { id = 2387, chance = 1500 }, -- double axe
    { id = 2568, chance = 9000 }, -- cleaver
    { id = 2260, chance = 11000 }, -- blank rune
  },
}
