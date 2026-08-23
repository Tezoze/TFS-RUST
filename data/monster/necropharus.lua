-- Generated from XML. Source: monsters/necropharus.xml
return {
  schema = 1,
  name = "Necropharus",
  description = "",
  race = "blood",
  experience = 700,
  speed = 60,
  mana_cost = 0,
  health = 750,
  max_health = 750,
  outfit = {
    look_type = 209,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 3058,
  },
  change_target = { chance = 50 },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  lose_target = { chance = 50 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = false,
    pushable = false,
    convinceable = false,
    can_push_items = true,
    can_push_creatures = true,
    target_distance = 4,
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 35,
      attack = 45,
      poison_cycles = 95,
      skill_factor = 1100,
      skill_next_level = 100,
      skill_add_count = 2,
    },
    {
      name = "poison",
      delay = 6,
      min = -50,
      max = -140,
      range = 7,
      shoot = "poison",
      effect = "poison",
    },
    {
      name = "lifedrain",
      delay = 5,
      min = -80,
      max = -120,
      range = 1,
      effect = "redspark",
    },
  },
  defenses = {
    armor = 55,
    defense = 45,
    spells = {
      {
        name = "healing",
        delay = 4,
        min = 60,
        max = 90,
        effect = "blueshimmer",
      },
    },
  },
  immunities = {
    fire = false,
    energy = false,
    poison = true,
    physical = false,
    outfit = true,
    life_drain = true,
    paralyze = false,
    invisible = true,
  },
  voices = {
    { text = "You will rise as my servant!", yell = false },
    { text = "Praise to my master Urgith!", yell = false },
  },
  summons = {
    max = 2,
    { name = "Mummy", delay = 7, max = 1 },
    { name = "Ghost", delay = 6, max = 1 },
    { name = "Ghoul", delay = 5, max = 2 },
  },
  loot = {
    { id = 2436, chance = 400 }, -- skull staff
    { id = 2229, chance = 16000 }, -- skull
    { id = 2406, chance = 8600 }, -- short sword
    { id = 2483, chance = 8500 }, -- scale armor
    { id = 2663, chance = 1800 }, -- mystic turban
    { id = 2796, chance = 22500 }, -- green mushroom
    { id = 2148, chance = 67300, count_max = 99 }, -- gold coin
    { id = 2186, chance = 500 }, -- moonlight rod
    { id = 2423, chance = 5700 }, -- clerical mace
    { id = 2195, chance = 200 }, -- boots of haste
    { id = 2541, chance = 7500 }, -- bone shield
    { id = 2449, chance = 19900 }, -- bone club
    { id = 2230, chance = 30000 }, -- bone
    { id = 2231, chance = 6000 }, -- big bone
  },
}
