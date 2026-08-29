-- Native implementations: `crates/tfs-rust-lua/src/userdata/player.rs` (CreatureRef).

function Creature.getClosestFreePosition(self, position, maxRadius, mustBeReachable)
	error("Creature.getClosestFreePosition is native-only")
end

function Creature.getPlayer(self)
	error("Creature.getPlayer is native-only")
end

function Creature.isContainer(self)
	return false
end

function Creature.isItem(self)
	return false
end

function Creature.isMonster(self)
	return false
end

function Creature.isNpc(self)
	return false
end

function Creature.isPlayer(self)
	return false
end

function Creature.isTeleport(self)
	return false
end

function Creature.isTile(self)
	return false
end

function Creature:setMonsterOutfit(monster, time)
	error("Creature.setMonsterOutfit is native-only")
end

function Creature:setItemOutfit(item, time)
	error("Creature.setItemOutfit is native-only")
end

function Creature:addSummon(monster)
	error("Creature.addSummon is native-only")
end

function Creature:removeSummon(monster)
	error("Creature.removeSummon is native-only")
end

function Creature:addDamageCondition(target, type, list, damage, period, rounds)
	error("Creature.addDamageCondition is native-only")
end

function Creature:canAccessPz()
	error("Creature.canAccessPz is native-only")
end
