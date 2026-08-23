-- Generated from XML. Source: monsters/priestess.xml
return {
  schema = 1,
  name = "Priestess",
  description = "a priestess",
  race = "blood",
  experience = 420,
  speed = 45,
  mana_cost = 0,
  health = 390,
  max_health = 390,
  outfit = {
    look_type = 58,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 3065,
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
    can_push_creatures = true,
    target_distance = 4,
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 25,
      attack = 20,
      poison_cycles = 250,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "manadrain",
      delay = 4,
      min = -60,
      max = -120,
      range = 7,
    },
    {
      name = "physical",
      delay = 4,
      min = -55,
      max = -95,
      range = 7,
      shoot = "death",
    },
  },
  defenses = {
    armor = 30,
    defense = 50,
    spells = {
      {
        name = "healing",
        delay = 7,
        min = 34,
        max = 56,
        effect = "blueshimmer",
      },
    },
  },
  immunities = {
    fire = true,
    energy = true,
    poison = true,
    physical = false,
    outfit = false,
    life_drain = false,
    paralyze = false,
    invisible = true,
  },
  voices = {
    { text = "Your energy is mine.", yell = false },
    { text = "Now, your life has come to an end, hahahha!", yell = false },
    { text = "Throw the soul on the altar!", yell = false },
  },
  summons = {
    max = 2,
    { name = "Ghoul", delay = 13, max = 2 },
  },
  loot = {
    { id = 2070, chance = 1400 }, -- wooden flute
    { id = 2791, chance = 3500 }, -- wood mushroom
    { id = 2183, chance = 1000 }, -- tempest rod
    { id = 2151, chance = 700 }, -- talon
    { id = 2802, chance = 14000 }, -- sling herb
    { id = 2674, chance = 7500, count_max = 2 }, -- red apple
    { id = 2803, chance = 6000 }, -- powder herb
    { id = 2760, chance = 12000 }, -- goat grass
    { id = 2379, chance = 23000 }, -- dagger
    { id = 2125, chance = 600 }, -- crystal necklace
    { id = 2192, chance = 1200 }, -- crystal ball
    { id = 2423, chance = 1500 }, -- clerical mace
    { id = 2032, chance = 20000 }, -- bowl
    { id = 1977, chance = 7000 }, -- book
    { id = 2529, chance = 200 }, -- black shield
  },
}
