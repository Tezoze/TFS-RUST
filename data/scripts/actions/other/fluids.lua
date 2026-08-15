-- 772 UseLiquidContainer (`moveuse.cc:1692-1818`).
-- Fill LIQUIDSOURCE → pour empty dest container → drink iff dest is self → else spill 2016.
-- Drink: beer/wine stack drunk; slime POISON_PERIODIC 200/3/3; mana 50–150; life 25–75.

local items = {
	1775, 2005, 2006, 2007, 2008, 2009,
	2011, 2012, 2013, 2014, 2015, 2023,
	2031, 2032, 2033, 2034, 2574, 2575,
	2576, 2577, 2562
}

local drunk = Condition(CONDITION_DRUNK)

local poison = Condition(CONDITION_POISON)
poison:setParameter(CONDITION_PARAM_CYCLE, 200)
poison:setParameter(CONDITION_PARAM_COUNT, 3)
poison:setParameter(CONDITION_PARAM_MAX_COUNT, 3)

local function destIsSelf(player, target, toPosition)
	if type(target.isCreature) == "function" and target:isCreature() then
		return target:getId() == player:getId()
	end
	local tile = Tile(toPosition)
	if not tile then
		return false
	end
	local creature = tile:getBottomCreature()
	return creature ~= nil and creature:getId() == player:getId()
end

local action = Action()

function action.onUse(player, item, fromPosition, target, toPosition, isHotkey)
	local fluidType = item:getFluidType()

	if target.itemid and target.itemid >= 100 then
		local destType = ItemType(target.itemid)
		if destType:getFluidSource() ~= FLUID_NONE and fluidType == FLUID_NONE then
			item:transform(item:getId(), destType:getFluidSource())
			return true
		end
		if fluidType ~= FLUID_NONE and target:getFluidType() == FLUID_NONE and destType:isFluidContainer() then
			target:transform(target:getId(), fluidType)
			item:transform(item:getId(), FLUID_NONE)
			return true
		end
	end

	if destIsSelf(player, target, toPosition) or isHotkey then
		if fluidType == FLUID_NONE then
			return true
		end
		if fluidType == FLUID_BEER or fluidType == FLUID_WINE then
			player:addCondition(drunk)
			player:say("Aah...", TALKTYPE_SAY)
		elseif fluidType == FLUID_SLIME then
			player:addCondition(poison)
			player:say("Urgh!", TALKTYPE_SAY)
		elseif fluidType == FLUID_URINE then
			player:say("Urgh!", TALKTYPE_SAY)
		elseif fluidType == FLUID_MANAFLUID then
			player:addMana(math.random(50, 150))
			player:say("Aaaah...", TALKTYPE_SAY)
		elseif fluidType == FLUID_LIFEFLUID then
			player:addHealth(math.random(25, 75))
			player:say("Aaaah...", TALKTYPE_SAY)
		elseif fluidType == FLUID_LEMONADE then
			player:say("Mmmh.", TALKTYPE_SAY)
		else
			player:say("Gulp.", TALKTYPE_SAY)
		end
		item:transform(item:getId(), FLUID_NONE)
		return true
	end

	if fluidType == FLUID_NONE then
		return false
	end

	local spillPos = toPosition
	if spillPos.x == 0xFFFF then
		spillPos = player:getPosition()
	end
	local splash = Game.createItem(2016, fluidType, spillPos)
	if splash then
		splash:decay()
	end
	item:transform(item:getId(), FLUID_NONE)
	return true
end

for _, id in ipairs(items) do
	action:id(id)
end

action:register()
