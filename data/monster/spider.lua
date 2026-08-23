-- Generated from XML. Source: monsters/spider.xml
return {
  schema = 1,
  name = "Spider",
  description = "a spider",
  race = "venom",
  experience = 12,
  speed = 36,
  mana_cost = 210,
  health = 20,
  max_health = 20,
  outfit = {
    look_type = 30,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2807,
  },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  flags = {
    hostile = true,
    summonable = true,
    illusionable = true,
    pushable = true,
    convinceable = true,
    can_push_items = false,
    can_push_creatures = false,
    target_distance = 1,
    run_health = 6,
  },
  attacks = {
    {
      name = "melee",
      skill = 19,
      attack = 7,
      skill_factor = 1000,
      skill_next_level = 50,
      skill_add_count = 2,
    },
  },
  defenses = {
    armor = 2,
    defense = 2,
  },
  immunities = {
    fire = false,
    energy = false,
    poison = false,
    physical = false,
    outfit = false,
    life_drain = false,
    paralyze = false,
    invisible = false,
  },
  loot = {
    { id = 2148, chance = 35000, count_max = 5 }, -- gold coin
  },
}
