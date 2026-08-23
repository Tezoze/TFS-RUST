-- Generated from XML. Source: monsters/orc warlord.xml
return {
  schema = 1,
  name = "Orc Warlord",
  description = "an orc warlord",
  race = "blood",
  experience = 670,
  speed = 77,
  mana_cost = 0,
  health = 950,
  max_health = 950,
  outfit = {
    look_type = 2,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2967,
  },
  change_target = { chance = 5 },
  target_strategy = { nearest = 70, weakest = 15, most_damage = 15, random = 0 },
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
      skill = 72,
      attack = 68,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "physical",
      delay = 4,
      min = -100,
      max = -120,
      range = 7,
      shoot = "throwingstar",
    },
  },
  defenses = {
    armor = 28,
    defense = 55,
    spells = {
      {
        name = "invisible",
        delay = 25,
        duration = 2000,
        effect = "blueshimmer",
      },
    },
  },
  immunities = {
    fire = true,
    energy = false,
    poison = false,
    physical = false,
    outfit = true,
    life_drain = false,
    paralyze = false,
    invisible = true,
  },
  voices = {
    { text = "Ranat Ulderek!", yell = false },
    { text = "Orc buta bana!", yell = false },
    { text = "Ikem rambo zambo!", yell = false },
    { text = "Futchi maruk buta!", yell = false },
  },
  loot = {
    { id = 2377, chance = 2000 }, -- two handed sword
    { id = 2399, chance = 30000, count_max = 40 }, -- throwing star
    { id = 2165, chance = 100 }, -- stealth ring
    { id = 2419, chance = 12000 }, -- scimitar
    { id = 2200, chance = 2000 }, -- protection amulet
    { id = 2647, chance = 4000 }, -- plate legs
    { id = 2463, chance = 6000 }, -- plate armor
    { id = 2428, chance = 15000 }, -- orcish axe
    { id = 2666, chance = 20000, count_max = 2 }, -- meat
    { id = 2148, chance = 19000, count_max = 45 }, -- gold coin
    { id = 2667, chance = 10000, count_max = 2 }, -- fish
    { id = 2434, chance = 200 }, -- dragon hammer
    { id = 2490, chance = 1500 }, -- dark helmet
    { id = 2497, chance = 200 }, -- crusader helmet
    { id = 2478, chance = 10000 }, -- brass legs
    { id = 2465, chance = 1000 }, -- brass armor
  },
}
