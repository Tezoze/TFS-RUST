-- Generated from XML. Source: monsters/hero.xml
return {
  schema = 1,
  name = "Hero",
  description = "a hero",
  race = "blood",
  experience = 1200,
  speed = 100,
  mana_cost = 0,
  health = 1400,
  max_health = 1400,
  outfit = {
    look_type = 73,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4240,
  },
  change_target = { chance = 5 },
  target_strategy = { nearest = 80, weakest = 10, most_damage = 10, random = 0 },
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
      skill = 80,
      attack = 58,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "physical",
      delay = 4,
      min = -75,
      max = -125,
      range = 7,
      shoot = "arrow",
    },
  },
  defenses = {
    armor = 35,
    defense = 50,
    spells = {
      {
        name = "healing",
        delay = 10,
        min = 200,
        max = 250,
        effect = "blueshimmer",
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
    { text = "Let's have a fight!", yell = false },
    { text = "Welcome to my battleground.", yell = false },
    { text = "Have you seen princess Lumelia?", yell = false },
    { text = "I will sing a tune at your grave.", yell = false },
  },
  loot = {
    { id = 3004, chance = 5000 }, -- wedding ring
    { id = 3279, chance = 1000 }, -- war hammer
    { id = 3265, chance = 1500 }, -- two handed sword
    { id = 2815, chance = 45000 }, -- scroll
    { id = 3572, chance = 12000 }, -- scarf
    { id = 3003, chance = 20000 }, -- rope
    { id = 3658, chance = 20000 }, -- red rose
    { id = 3048, chance = 500 }, -- might ring
    { id = 3577, chance = 18000, count_max = 2 }, -- meat
    { id = 2949, chance = 15000 }, -- lyre
    { id = 3563, chance = 8000 }, -- green tunic
    { id = 3592, chance = 20000 }, -- grapes
    { id = 3031, chance = 60000, count_max = 100 }, -- gold coin
    { id = 3280, chance = 500 }, -- fire sword
    { id = 3419, chance = 400 }, -- crown shield
    { id = 3382, chance = 500 }, -- crown legs
    { id = 3385, chance = 500 }, -- crown helmet
    { id = 3381, chance = 600 }, -- crown armor
    { id = 3350, chance = 13000 }, -- bow
    { id = 3447, chance = 27000, count_max = 13 }, -- arrow
  },
}
