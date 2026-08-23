function Container.isContainer(self)
	return true
end

function Container.createLootItem(self, item)
	error("createLootItem: loot is rolled natively at spawn")
end
