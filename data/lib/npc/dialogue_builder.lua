-- DialogueBuilder: Specialized NPC builder for keyword-response NPCs
-- Extends NpcBuilder with a convenience method for adding keyword-response pairs.
-- Supports multi-keyword matching (e.g., both "job" and "work" trigger same response).
-- Voice support inherited from NpcBuilder (addVoice, voiceInterval, voiceChance).

DialogueBuilder = setmetatable({}, { __index = NpcBuilder })
DialogueBuilder.__index = DialogueBuilder

function DialogueBuilder:new(name, outfit)
    local obj = NpcBuilder.new(self, name, outfit)
    setmetatable(obj, self)
    return obj
end

-- Convenience: add multiple keywords mapping to same response
function DialogueBuilder:addResponse(keywords, text)
    -- keywords can be {"job", "work"} to match either
    if type(keywords) == "string" then keywords = {keywords} end
    for _, kw in ipairs(keywords) do
        self._keywords:addKeyword({kw}, function(npc, player, message, builder)
            if not builder:isFocused(npc:getId(), player:getId()) then return false end
            local msg = text:gsub("|PLAYERNAME|", player:getName())
            builder:say(npc, msg, player)
            InstanceState.updateTalkStart(npc:getId(), player:getId())
            return true
        end)
    end
    return self
end
