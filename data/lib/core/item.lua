function Item.getType(self)
	return ItemType(self:getId())
end

function Item.isContainer(self)
	return false
end

function Item.isCreature(self)
	return false
end

function Item.isMonster(self)
	return false
end

function Item.isNpc(self)
	return false
end

function Item.isPlayer(self)
	return false
end

function Item.isTeleport(self)
	return false
end

function Item.isTile(self)
	return false
end

function Item.getDescription(self, lookDistance, subType)
	error("Item.getDescription is native-only")
end

function Item.getNameDescription(self, subType, addArticle)
	error("Item.getNameDescription is native-only")
end

function ItemType.getDescription(self, lookDistance, subType)
	error("ItemType.getDescription is native-only")
end

function ItemType.getNameDescription(self, subType, addArticle)
	error("ItemType.getNameDescription is native-only")
end
