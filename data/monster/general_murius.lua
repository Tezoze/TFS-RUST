-- Generated from XML. Source: monsters/general murius.xml
return {
  schema = 1,
  name = "General Murius",
  description = "",
  race = "blood",
  experience = 300,
  speed = 85,
  mana_cost = 0,
  health = 550,
  max_health = 550,
  outfit = {
    look_type = 207,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2876,
  },
  change_target = { chance = 5 },
  target_strategy = { nearest = 70, weakest = 10, most_damage = 20, random = 0 },
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
      skill = 70,
      attack = 55,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "physical",
      delay = 10,
      min = -50,
      max = -80,
      radius = 3,
      target = false,
      effect = "blackspark",
    },
    {
      name = "physical",
      delay = 9,
      min = -50,
      max = -120,
      range = 7,
      shoot = "bolt",
    },
  },
  defenses = {
    armor = 26,
    defense = 52,
    spells = {
      {
        name = "healing",
        delay = 7,
        min = 50,
        max = 100,
        effect = "redshimmer",
      },
    },
  },
  immunities = {
    fire = false,
    energy = false,
    poison = false,
    physical = false,
    outfit = true,
    life_drain = true,
    paralyze = false,
    invisible = true,
  },
  voices = {
    { text = "Feel the power of the Mooh'Tah!", yell = false },
    { text = "You will get what you deserve!", yell = false },
    { text = "For the king!", yell = false },
    { text = "Guards!", yell = false },
  },
  summons = {
    max = 2,
    { name = "Minotaur Guard", delay = 9, max = 2 },
    { name = "Minotaur Archer", delay = 7, max = 2 },
  },
  loot = {
    { id = 2666, chance = 10000 }, -- meat
    { id = 2148, chance = 60000, count_max = 50 }, -- gold coin
    { id = 2580, chance = 5000 }, -- fishing rod
    { id = 2387, chance = 7500 }, -- double axe
    { id = 2648, chance = 35000 }, -- chain legs
    { id = 2465, chance = 28000 }, -- brass armor
    { id = 2513, chance = 18000 }, -- battle shield
  },
}
