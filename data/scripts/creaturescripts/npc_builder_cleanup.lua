-- NPC Builder Cleanup: Automatic state cleanup on player logout/disconnect
-- Must be in data/scripts/ (Scripts Interface) because CreatureEvent() requires it.

-- Logout event: clean up all NPC builder state for this player
local npcCleanup = CreatureEvent("NpcBuilderCleanup")

function npcCleanup.onLogout(player)
    if InstanceState then
        InstanceState.removePlayer(player:getId())
    end
    return true
end

npcCleanup:type("logout")
npcCleanup:register()

-- Login event: register the cleanup event on each player
local npcCleanupLogin = CreatureEvent("NpcBuilderCleanupLogin")

function npcCleanupLogin.onLogin(player)
    player:registerEvent("NpcBuilderCleanup")
    return true
end

npcCleanupLogin:type("login")
npcCleanupLogin:register()
