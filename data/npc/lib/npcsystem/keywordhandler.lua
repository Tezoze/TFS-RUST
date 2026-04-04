-- Advanced NPC System by Jiddo

if not KeywordHandler then
	KeywordNode = {
		keywords = nil,
		callback = nil,
		parameters = nil,
		children = nil,
		parent = nil
	}

	-- Created a new keywordnode with the given keywords, callback function and parameters and without any childNodes.
	function KeywordNode:new(keys, func, param)
		local obj = {}
		obj.keywords = keys
		obj.callback = func
		obj.parameters = param
		obj.children = {}
		setmetatable(obj, self)
		self.__index = self
		return obj
	end

	-- Calls the underlying callback function if it is not nil.
	function KeywordNode:processMessage(cid, message)
		return (not self.callback or self.callback(cid, message, self.keywords, self.parameters, self))
	end

	-- Returns true if message contains all patterns/strings found in keywords.
	function KeywordNode:checkMessage(message)
		if self.keywords.callback then
			return self.keywords.callback(self.keywords, message)
		end

		for _, v in ipairs(self.keywords) do
			if type(v) == 'string' then
				local a, b = string.find(message, v)
				if not a or not b then
					return false
				end
			end
		end
		return true
	end

	-- Returns the parent of this node or nil if no such node exists.
	function KeywordNode:getParent()
		return self.parent
	end

	-- Returns an array of the callback function parameters assosiated with this node.
	function KeywordNode:getParameters()
		return self.parameters
	end

	-- Returns an array of the triggering keywords assosiated with this node.
	function KeywordNode:getKeywords()
		return self.keywords
	end

	-- Adds a childNode to this node. Creates the childNode based on the parameters (k = keywords, c = callback, p = parameters, ...options)
	function KeywordNode:addChildKeyword(keywords, callback, parameters, ...)
		local new = KeywordNode:new(keywords, callback, parameters)
		local args = {...}
		if #args > 0 then
			new.action = args[#args]  -- Last argument is assumed to be the action
		end
		return self:addChildKeywordNode(new)
	end

	-- Adds an alias keyword. Should be used if you have to answer the same thing to several keywords.
	function KeywordNode:addAliasKeyword(keywords)
		if #self.children == 0 then
			print('KeywordNode:addAliasKeyword no previous node found')
			return false
		end

		local prevNode = self.children[#self.children]
		local new = KeywordNode:new(keywords, prevNode.callback, prevNode.parameters)
		new.action = prevNode.action
		for i = 1, #prevNode.children do
			new:addChildKeywordNode(prevNode.children[i])
		end
		return self:addChildKeywordNode(new)
	end

	-- Adds a pre-created childNode to this node. Should be used for example if several nodes should have a common child.
	function KeywordNode:addChildKeywordNode(childNode)
		self.children[#self.children + 1] = childNode
		childNode.parent = self
		return childNode
	end

	KeywordHandler = {
		root = nil,
		lastNode = nil
	}

	-- Creates a new keywordhandler with an empty rootnode.
	function KeywordHandler:new()
		local obj = {}
		obj.root = KeywordNode:new(nil, nil, nil)
		obj.lastNode = {}
		setmetatable(obj, self)
		self.__index = self
		return obj
	end

	-- Resets the lastNode field, and this resetting the current position in the node hierarchy to root.
	function KeywordHandler:reset(cid)
		if self.lastNode[cid] then
			self.lastNode[cid] = nil
		end
	end

	-- Makes sure the correct childNode of lastNode gets a chance to process the message.
	function KeywordHandler:processMessage(cid, message)
		local node = self:getLastNode(cid)
		if not node then
			error('No root node found.')
			return false
		end

		local ret = self:processNodeMessage(node, cid, message)
		if ret then
			return true
		end

		if node:getParent() then
			node = node:getParent() -- Search through the parent.
			local ret = self:processNodeMessage(node, cid, message)
			if ret then
				return true
			end
		end

		if node ~= self:getRoot() then
			node = self:getRoot() -- Search through the root.
			local ret = self:processNodeMessage(node, cid, message)
			if ret then
				return true
			end
		end
		return false
	end

	-- Tries to process the given message using the node parameter's children and calls the node's callback function if found.
	--	Returns the childNode which processed the message or nil if no such node was found.
	function KeywordHandler:processNodeMessage(node, cid, message)
		local messageLower = string.lower(message)
		
		-- Collect all matching nodes with their keyword count (specificity)
		local matches = {}
		for i, childNode in pairs(node.children) do
			if childNode:checkMessage(messageLower) then
				local keywordCount = 0
				if childNode.keywords and type(childNode.keywords) == "table" then
					keywordCount = #childNode.keywords
				end
				table.insert(matches, {node = childNode, specificity = keywordCount, index = i})
			end
		end
		
		-- Sort by specificity (more keywords = more specific = higher priority)
		table.sort(matches, function(a, b)
			return a.specificity > b.specificity
		end)
		
		-- Try matches in order of specificity (most specific first)
		for _, match in ipairs(matches) do
			local childNode = match.node
			local oldLast = self.lastNode[cid]
			self.lastNode[cid] = childNode
			childNode.parent = node -- Make sure node is the parent of childNode (as one node can be parent to several nodes).
			if childNode:processMessage(cid, message) then
				return true
			end
			self.lastNode[cid] = oldLast
		end
		return false
	end

	-- Returns the root keywordnode
	function KeywordHandler:getRoot()
		return self.root
	end

	-- Returns the last processed keywordnode or root if no last node is found.
	function KeywordHandler:getLastNode(cid)
		return self.lastNode[cid] or self:getRoot()
	end

	-- Adds a new keyword to the root keywordnode. Returns the new node.
	function KeywordHandler:addKeyword(keys, callback, parameters)
		return self:getRoot():addChildKeyword(keys, callback, parameters)
	end

	-- Adds an alias keyword for the previous node.
	function KeywordHandler:addAliasKeyword(keys)
		return self:getRoot():addAliasKeyword(keys)
	end

	-- Moves the current position in the keyword hierarchy steps upwards. Steps defalut value = 1.
	function KeywordHandler:moveUp(cid, steps)
		if not steps or type(steps) ~= "number" then
			steps = 1
		end

		for i = 1, steps do
			if not self.lastNode[cid] then
				return nil
			end
			self.lastNode[cid] = self.lastNode[cid]:getParent() or self:getRoot()
		end
		return self.lastNode[cid]
	end

	-- Compatibility method for old NPC spell system
	function KeywordHandler:addSpellKeyword(keywords, parameters)
		local npcHandler = parameters.npcHandler
		if not npcHandler then
			return
		end

		local spellName = parameters.spellName
		local price = parameters.price or 0
		local level = parameters.level or 0
		local vocation = parameters.vocation

		-- Add keyword for spell learning/teaching
		local node = self:addKeyword(keywords, function(cid, message, keywords, parameters, node)
			local player = Player(cid)
			if not player then
				return false
			end

			-- Check level requirement
			if player:getLevel() < level then
				npcHandler:say("You need to be at least level " .. level .. " to learn this spell.", cid)
				return true
			end

			-- Check vocation requirement
			if vocation and type(vocation) == "table" then
				local playerVocation = player:getVocation():getId()
				local allowed = false
				for _, v in ipairs(vocation) do
					if playerVocation == v then
						allowed = true
						break
					end
				end
				if not allowed then
					npcHandler:say("This spell is not available for your vocation.", cid)
					return true
				end
			end

			-- Check if player already knows the spell
			if player:hasLearnedSpell(spellName) then
				npcHandler:say("You already know this spell.", cid)
				return true
			end

			-- Check if player can afford it
			if not player:removeTotalMoney(price) then
				npcHandler:say("You don't have enough money. This spell costs " .. price .. " gold coins.", cid)
				return true
			end

			-- Teach the spell
			player:learnSpell(spellName)
			npcHandler:say("You have learned " .. spellName .. "!", cid)
			return true
		end, parameters)

		return node
	end

	-- Compatibility method for old NPC greet system
	function KeywordHandler:addGreetKeyword(keywords, parameters)
		local npcHandler = parameters.npcHandler
		if not npcHandler then
			return
		end

		local text = parameters.text
		local node = self:addKeyword(keywords, function(cid, message, keywords, parameters, node)
			local npcHandler = parameters.npcHandler
			if npcHandler then
				npcHandler:say(parameters.text, cid)
				npcHandler:onGreet(cid)
			end
			return true
		end, parameters)

		return node
	end

	-- Compatibility method for old NPC farewell system
	function KeywordHandler:addFarewellKeyword(keywords, parameters)
		local npcHandler = parameters.npcHandler
		if not npcHandler then
			return
		end

		local text = parameters.text
		local node = self:addKeyword(keywords, function(cid, message, keywords, parameters, node)
			local npcHandler = parameters.npcHandler
			if npcHandler then
				npcHandler:say(parameters.text, cid)
				npcHandler:onFarewell(cid)
			end
			return true
		end, parameters)

		return node
	end
end
