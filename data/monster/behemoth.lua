-- Generated from XML. Source: monsters/behemoth.xml
return {
  schema = 1,
  name = "Behemoth",
  description = "a behemoth",
  race = "blood",
  experience = 2500,
  speed = 130,
  mana_cost = 0,
  health = 4000,
  max_health = 4000,
  outfit = {
    look_type = 55,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2931,
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
      skill = 110,
      attack = 75,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "physical",
      delay = 7,
      min = -125,
      max = -185,
      range = 7,
      shoot = "largerock",
    },
  },
  defenses = {
    armor = 50,
    defense = 65,
    spells = {
      {
        name = "speed",
        delay = 15,
        duration = 8000,
        speed = 45,
        speed_variation = 5,
        effect = "redshimmer",
      },
    },
  },
  immunities = {
    fire = true,
    energy = true,
    poison = true,
    physical = false,
    outfit = true,
    life_drain = false,
    paralyze = true,
    invisible = true,
  },
  voices = {
    { text = "You're so little!", yell = false },
    { text = "Human flesh - delicious!", yell = false },
    { text = "Crush the intruders!", yell = true },
  },
  loot = {
    { id = 2377, chance = 4000 }, -- two handed sword
    { id = 2174, chance = 800 }, -- strange symbol
    { id = 2645, chance = 400 }, -- steel boots
    { id = 2150, chance = 4000, count_max = 2 }, -- small amethyst
    { id = 2463, chance = 2000 }, -- plate armor
    { id = 2553, chance = 6000 }, -- pick
    { id = 2666, chance = 40000, count_max = 6 }, -- meat
    { id = 2148, chance = 50000, count_max = 80 }, -- gold coin
    { id = 2148, chance = 70000, count_max = 60 }, -- gold coin
    { id = 2393, chance = 1000 }, -- giant sword
    { id = 2387, chance = 10000 }, -- double axe
    { id = 2489, chance = 3000 }, -- dark armor
    { id = 2125, chance = 300 }, -- crystal necklace
    { id = 2416, chance = 15000 }, -- crowbar
    { id = 2231, chance = 7000 }, -- big bone
    { id = 2023, chance = 11000 }, -- amphora
  },
}
