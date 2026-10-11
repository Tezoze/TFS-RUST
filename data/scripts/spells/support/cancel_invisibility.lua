local combat = Combat()
combat:setParameter(COMBAT_PARAM_EFFECT, CONST_ME_MAGIC_BLUE)
combat:setParameter(COMBAT_PARAM_AGGRESSIVE, false)
combat:setArea(createCombatArea(AREA_CIRCLE3X3))

local spell = Spell(SPELL_INSTANT)

-- `CancelInvisibility` (`magic.cc:2381-2448`): ring starts at 1, so the caster's
-- tile is skipped. Protection zones are skipped. On a no-PvP world a peaceful
-- creature (player, npc, or a player's summon) stays hidden.
local function sameTile(a, b)
	return a.x == b.x and a.y == b.y and a.z == b.z
end

local function isPeaceful(creature)
	if creature:isPlayer() or creature:isNpc() then
		return true
	end
	if creature:isMonster() then
		local master = creature:getMaster()
		return master ~= nil and master:isPlayer()
	end
	return false
end

function onTargetCreature(creature, target)
	if target == creature then
		return true
	end
	local origin = creature:getPosition()
	local pos = target:getPosition()
	if sameTile(origin, pos) then
		return true
	end
	local tile = Tile(pos)
	if tile and tile:hasFlag(TILESTATE_PROTECTIONZONE) then
		return true
	end
	if Game.getWorldType() == WORLD_TYPE_NO_PVP and isPeaceful(target) then
		return true
	end
	if not target:getCondition(CONDITION_INVISIBLE, CONDITIONID_DEFAULT, 0) then
		return true
	end
	if target:isPlayer() then
		Game.tryBreakIllusionItem(target)
	end
	target:removeCondition(CONDITION_INVISIBLE)
	return true
end

combat:setCallback(CALLBACK_PARAM_TARGETCREATURE, "onTargetCreature")

function spell.onCastSpell(creature, variant)
	return combat:execute(creature, variant)
end

spell:mana(200)
spell:level(26)
spell:isPremium(true)
spell:isAggressive(false)
spell:name("Cancel Invisibility")
spell:vocation("Sorcerer", "Master Sorcerer")
spell:words("ex,ana, ina")
spell:register()
