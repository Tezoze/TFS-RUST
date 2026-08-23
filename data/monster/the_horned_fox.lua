-- Generated from XML. Source: monsters/the horned fox.xml
return {
  schema = 1,
  name = "The Horned Fox",
  description = "",
  race = "blood",
  experience = 200,
  speed = 65,
  mana_cost = 0,
  health = 265,
  max_health = 265,
  outfit = {
    look_type = 202,
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
      skill = 54,
      attack = 38,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "poisoncondition",
      delay = 6,
      range = 7,
      cycle = 85,
      min_cycle = 35,
      shoot = "bolt",
    },
    {
      name = "physical",
      delay = 4,
      min = -50,
      max = -120,
      range = 7,
      shoot = "bolt",
    },
  },
  defenses = {
    armor = 17,
    defense = 36,
    spells = {
      {
        name = "invisible",
        delay = 11,
        duration = 2000,
        effect = "blueshimmer",
      },
      {
        name = "healing",
        delay = 7,
        min = 25,
        max = 75,
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
    life_drain = false,
    paralyze = false,
    invisible = true,
  },
  voices = {
    { text = "You will never get me!", yell = false },
    { text = "I'll be back!", yell = false },
    { text = "Catch me, if you can!", yell = false },
    { text = "Help me, boys!", yell = false },
  },
  summons = {
    max = 2,
    { name = "Minotaur Guard", delay = 8, max = 2 },
    { name = "Minotaur Archer", delay = 8, max = 2 },
  },
  loot = {
    { id = 2666, chance = 10000 }, -- meat
    { id = 2388, chance = 9000 }, -- hatchet
    { id = 2148, chance = 60000, count_max = 20 }, -- gold coin
    { id = 2580, chance = 5000 }, -- fishing rod
    { id = 2502, chance = 9000 }, -- dwarfen helmet
    { id = 2387, chance = 1000 }, -- double axe
    { id = 2648, chance = 15000 }, -- chain legs
    { id = 2465, chance = 14000 }, -- brass armor
    { id = 2513, chance = 2000 }, -- battle shield
  },
}
