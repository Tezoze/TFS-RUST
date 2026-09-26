-- 772 Collision / Trap Damage (`moveuse.dat`):
-- slits Change only; blades/spikes Damage(1,60); bear !IsPeaceful then Change+30
-- else Change+poff; maw Change+Damage(2,30). No TILESTATE_PROTECTIONZONE skip.

local traps = {
	[2145] = { -- strange slits
		transformTo = 2146,
	},
	[2146] = { -- blades (xml decay 1511→1510)
		damage = {-60, -60}
	},
	[2148] = { -- spikes
		damage = {-60, -60}
	},
	[3482] = { -- bear
		transformTo = 3481,
		damage = {-30, -30},
		dontDamagePlayers = true,
	},
	[3944] = { -- jungle maw
		transformTo = 3945,
		damage = {-30, -30},
		type = COMBAT_EARTHDAMAGE
	},
}

local stepInTrap = MoveEvent()
local onAddItem = MoveEvent()

function stepInTrap.onStepIn(creature, item, position, fromPosition)
	local trap = traps[item.itemid]
	if not trap then
		return true
	end

	if trap.transformTo then
		item:transform(trap.transformTo)
		item:decay()
	end

	local applyDamage = trap.damage ~= nil
	if applyDamage and trap.dontDamagePlayers then
		if creature:isPlayer() or (creature:getMaster() and creature:getMaster():getPlayer()) then
			applyDamage = false
			position:sendMagicEffect(CONST_ME_POFF)
		end
	end

	if applyDamage then
		doTargetCombat(0, creature, trap.type or COMBAT_PHYSICALDAMAGE, trap.damage[1], trap.damage[2], CONST_ME_NONE, ORIGIN_NONE, not trap.type and true or false, false, false)
	end
	return true
end

function onAddItem.onAddItem(moveitem, tileitem, pos)
	local trap = traps[tileitem.itemid]
	if not trap then
		return true
	end

	if trap.transformTo then
		tileitem:transform(trap.transformTo)
		tileitem:decay()
	end
	pos:sendMagicEffect(CONST_ME_POFF)
end

stepInTrap:id(2145, 2146, 2148, 3482, 3944)
stepInTrap:register()

onAddItem:id(3482, 3944)
onAddItem:tileItem(true)
onAddItem:register()
