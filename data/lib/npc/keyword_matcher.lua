-- KeywordMatcher: Flat priority-sorted keyword matching engine
-- Uses plain-text case-insensitive matching via string.lower and string.find(plain=true).
-- Unlike the Jiddo KeywordHandler tree, this uses a flat list with priority ordering.

KeywordMatcher = {}
KeywordMatcher.__index = KeywordMatcher

function KeywordMatcher:new()
    local obj = { rules = {} }
    setmetatable(obj, self)
    return obj
end

function KeywordMatcher:addKeyword(keywords, callback, priority)
    -- keywords: table of strings (all must match)
    -- callback: function(npc, player, message, builder) -> bool
    -- priority: higher = checked first (default 0)
    self.rules[#self.rules + 1] = {
        keywords = keywords,
        callback = callback,
        priority = priority or 0
    }
    -- Keep sorted by priority descending
    table.sort(self.rules, function(a, b) return a.priority > b.priority end)
end

function KeywordMatcher:match(message, npc, player, builder)
    -- Returns true if a rule matched and handled the message
    local lower = message:lower()
    for _, rule in ipairs(self.rules) do
        local matched = true
        for _, kw in ipairs(rule.keywords) do
            if not lower:find(kw, 1, true) then
                matched = false
                break
            end
        end
        if matched then
            if rule.callback(npc, player, message, builder) then
                return true
            end
        end
    end
    return false
end
