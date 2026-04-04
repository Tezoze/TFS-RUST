-- Import storages
dofile("data/lib/core/storages.lua")

-- Configuration for enchantment system
local config = {
    equipment = {
        charges = 1000,
        effect = CONST_ME_MAGIC_RED,
        defaultMinLevel = 20,
        defaultMaxLevel = 30
    }
}

local items = {
    equipment = {
        [2147] = { -- small ruby
            [COMBAT_FIREDAMAGE] = {id = 2343, targetId = 2147} -- helmet of the ancients (enchanted)
        },
        [2383] = { -- spike sword
            [COMBAT_FIREDAMAGE] = {id = 7744, min = 20, max = 30}, [COMBAT_ICEDAMAGE] = {id = 7763, min = 20, max = 30},
            [COMBAT_EARTHDAMAGE] = {id = 7854, min = 20, max = 30}, [COMBAT_ENERGYDAMAGE] = {id = 7869, min = 20, max = 30}
        },
        [2391] = { -- war hammer
            [COMBAT_FIREDAMAGE] = {id = 7758, min = 20, max = 30}, [COMBAT_ICEDAMAGE] = {id = 7777, min = 20, max = 30},
            [COMBAT_EARTHDAMAGE] = {id = 7868, min = 20, max = 30}, [COMBAT_ENERGYDAMAGE] = {id = 7883, min = 20, max = 30}
        },
        [2423] = { -- clerical mace
            [COMBAT_FIREDAMAGE] = {id = 7754, min = 20, max = 30}, [COMBAT_ICEDAMAGE] = {id = 7773, min = 20, max = 30},
            [COMBAT_EARTHDAMAGE] = {id = 7864, min = 20, max = 30}, [COMBAT_ENERGYDAMAGE] = {id = 7879, min = 20, max = 30}
        },
        [2429] = { -- barbarian axe
            [COMBAT_FIREDAMAGE] = {id = 7749, min = 20, max = 30}, [COMBAT_ICEDAMAGE] = {id = 7768, min = 20, max = 30},
            [COMBAT_EARTHDAMAGE] = {id = 7859, min = 20, max = 30}, [COMBAT_ENERGYDAMAGE] = {id = 7874, min = 20, max = 30}
        },
        [2430] = { -- knight axe
            [COMBAT_FIREDAMAGE] = {id = 7750, min = 20, max = 30}, [COMBAT_ICEDAMAGE] = {id = 7769, min = 20, max = 30},
            [COMBAT_EARTHDAMAGE] = {id = 7860, min = 20, max = 30}, [COMBAT_ENERGYDAMAGE] = {id = 7875, min = 20, max = 30}
        },
        [2445] = { -- crystal mace
            [COMBAT_FIREDAMAGE] = {id = 7755, min = 20, max = 30}, [COMBAT_ICEDAMAGE] = {id = 7774, min = 20, max = 30},
            [COMBAT_EARTHDAMAGE] = {id = 7865, min = 20, max = 30}, [COMBAT_ENERGYDAMAGE] = {id = 7880, min = 20, max = 30}
        },
        [2454] = { -- war axe
            [COMBAT_FIREDAMAGE] = {id = 7753, min = 20, max = 30}, [COMBAT_ICEDAMAGE] = {id = 7772, min = 20, max = 30},
            [COMBAT_EARTHDAMAGE] = {id = 7863, min = 20, max = 30}, [COMBAT_ENERGYDAMAGE] = {id = 7878, min = 20, max = 30}
        },
        [7380] = { -- headchopper
            [COMBAT_FIREDAMAGE] = {id = 7752, min = 20, max = 30}, [COMBAT_ICEDAMAGE] = {id = 7771, min = 20, max = 30},
            [COMBAT_EARTHDAMAGE] = {id = 7862, min = 20, max = 30}, [COMBAT_ENERGYDAMAGE] = {id = 7877, min = 20, max = 30}
        },
        [7383] = { -- relic sword
            [COMBAT_FIREDAMAGE] = {id = 7745, min = 20, max = 30}, [COMBAT_ICEDAMAGE] = {id = 7764, min = 20, max = 30},
            [COMBAT_EARTHDAMAGE] = {id = 7855, min = 20, max = 30}, [COMBAT_ENERGYDAMAGE] = {id = 7870, min = 20, max = 30}
        },
        [7384] = { -- mystic blade
            [COMBAT_FIREDAMAGE] = {id = 7746, min = 20, max = 30}, [COMBAT_ICEDAMAGE] = {id = 7765, min = 20, max = 30},
            [COMBAT_EARTHDAMAGE] = {id = 7856, min = 20, max = 30}, [COMBAT_ENERGYDAMAGE] = {id = 7871, min = 20, max = 30}
        },
        [7389] = { -- heroic axe
            [COMBAT_FIREDAMAGE] = {id = 7751, min = 20, max = 30}, [COMBAT_ICEDAMAGE] = {id = 7770, min = 20, max = 30},
            [COMBAT_EARTHDAMAGE] = {id = 7861, min = 20, max = 30}, [COMBAT_ENERGYDAMAGE] = {id = 7876, min = 20, max = 30}
        },
        [7392] = { -- orcish maul
            [COMBAT_FIREDAMAGE] = {id = 7757, min = 20, max = 30}, [COMBAT_ICEDAMAGE] = {id = 7776, min = 20, max = 30},
            [COMBAT_EARTHDAMAGE] = {id = 7867, min = 20, max = 30}, [COMBAT_ENERGYDAMAGE] = {id = 7882, min = 20, max = 30}
        },
        [7402] = { -- dragon slayer
            [COMBAT_FIREDAMAGE] = {id = 7748, min = 20, max = 30}, [COMBAT_ICEDAMAGE] = {id = 7767, min = 20, max = 30},
            [COMBAT_EARTHDAMAGE] = {id = 7858, min = 20, max = 30}, [COMBAT_ENERGYDAMAGE] = {id = 7873, min = 20, max = 30}
        },
        [7406] = { -- blacksteel sword
            [COMBAT_FIREDAMAGE] = {id = 7747, min = 20, max = 30}, [COMBAT_ICEDAMAGE] = {id = 7766, min = 20, max = 30},
            [COMBAT_EARTHDAMAGE] = {id = 7857, min = 20, max = 30}, [COMBAT_ENERGYDAMAGE] = {id = 7872, min = 20, max = 30}
        },
        [7415] = { -- cranial basher
            [COMBAT_FIREDAMAGE] = {id = 7756, min = 20, max = 30}, [COMBAT_ICEDAMAGE] = {id = 7775, min = 20, max = 30},
            [COMBAT_EARTHDAMAGE] = {id = 7866, min = 20, max = 30}, [COMBAT_ENERGYDAMAGE] = {id = 7881, min = 20, max = 30}
        },
        [8905] = { -- rainbow shield
            [COMBAT_FIREDAMAGE] = {id = 8906, min = 180, max = 190}, [COMBAT_ICEDAMAGE] = {id = 8907, min = 180, max = 190},
            [COMBAT_EARTHDAMAGE] = {id = 8909, min = 180, max = 190}, [COMBAT_ENERGYDAMAGE] = {id = 8908, min = 180, max = 190}
        },
        [9949] = { -- dracoyle statue
            [COMBAT_EARTHDAMAGE] = {id = 9948} -- dracoyle statue (enchanted)
        },
        [9954] = { -- dracoyle statue
            [COMBAT_EARTHDAMAGE] = {id = 9953} -- dracoyle statue (enchanted)
        },
        [10022] = { -- worn firewalker boots
            [COMBAT_FIREDAMAGE] = {id = 9933, say = {text = "Quitate las botas antes de encantarlas."}},
            slot = {type = CONST_SLOT_FEET, check = true}
        },
        [24718] = { -- werewolf helmet
            [COMBAT_NONE] = {
                id = {
                    [SKILL_CLUB] = {id = 24783},
                    [SKILL_SWORD] = {id = 24783},
                    [SKILL_AXE] = {id = 24783},
                    [SKILL_DISTANCE] = {id = 24783},
                    [SKILL_MAGLEVEL] = {id = 24783}
                },
                effects = {failure = CONST_ME_POFF, success = CONST_ME_THUNDER},
                message = {text = "The helmet cannot be enchanted while worn."},
                usesStorage = true
            },
            slot = {type = CONST_SLOT_HEAD, check = true}
        }
    },
   
    valuables = {
        [2146] = {id = 7759, shrine = {7508, 7509, 7510, 7511}}, -- small sapphire
        [2147] = {id = 7760, shrine = {7504, 7505, 7506, 7507}}, -- small ruby
        [2149] = {id = 7761, shrine = {7516, 7517, 7518, 7519}}, -- small emerald
        [2150] = {id = 7762, shrine = {7512, 7513, 7514, 7515}}, -- small amethyst
        soul = 2, mana = 300, effect = CONST_ME_HOLYDAMAGE
    },
   
    [2342] = {combatType = COMBAT_FIREDAMAGE, targetId = 2147}, -- helmet of the ancients
    [7759] = {combatType = COMBAT_ICEDAMAGE}, -- small enchanted sapphire
    [7760] = {combatType = COMBAT_FIREDAMAGE}, -- small enchanted ruby
    [7761] = {combatType = COMBAT_EARTHDAMAGE}, -- small enchanted emerald
    [7762] = {combatType = COMBAT_ENERGYDAMAGE}, -- small enchanted amethyst
}

-- Storage mapping for special items (werewolf helmet)
local storages = {
    [24718] = { -- werewolf helmet
        [24718] = { -- targetId
            [1] = {key = 50000}, -- knight vocation
            [2] = {key = 50001}, -- paladin vocation
            [3] = {key = 50002}, -- sorcerer vocation
            [4] = {key = 50003}, -- druid vocation
        }
    }
}

-- Helper function to apply enchantment levels and attributes
local function applyEnchantmentLevel(target, targetItem, itemId)
    -- This function is a placeholder for future enchantment level logic
    -- In this TFS version, we just do basic transformation without advanced features
end

function onUse(player, item, fromPosition, target, toPosition, isHotkey)
    if not target or not target:isItem() then
        return false
    end
   
    local itemId, targetId = item:getId(), target:getId()
    local targetType = items.valuables[itemId] or items.equipment[items[itemId] and items[itemId].targetId or targetId]
    if not targetType then
        player:sendCancelMessage(RETURNVALUE_NOTPOSSIBLE)
        return true
    end
   
    if targetType.shrine then
        if not table.contains(targetType.shrine, targetId) then
            player:sendCancelMessage(RETURNVALUE_NOTPOSSIBLE)
            return true
        end
   
        if player:getMana() < items.valuables.mana then
            player:sendCancelMessage(RETURNVALUE_NOTENOUGHMANA)
            return true
        end
   
        if player:getSoul() < items.valuables.soul then
            player:sendCancelMessage(RETURNVALUE_NOTENOUGHSOUL)
            return true
        end
        player:addSoul(-items.valuables.soul)
        player:addMana(-items.valuables.mana)
        player:addManaSpent(items.valuables.mana)
        player:addItem(targetType.id)
        player:getPosition():sendMagicEffect(items.valuables.effect)
        --player:sendSupplyUsed(item)
        item:remove(1)
    else
        local targetItem = targetType[items[itemId] and items[itemId].combatType]
        if not targetItem or (targetItem.targetId and targetItem.targetId ~= targetId) then
            player:sendCancelMessage(RETURNVALUE_NOTPOSSIBLE)
            return true
        end
   
        local isInSlot = targetType.slot and targetType.slot.check and target:getType():usesSlot(targetType.slot.type) and Player(target:getParent())
        if isInSlot then
            if targetItem.say then
                player:say(targetItem.say.text, TALKTYPE_MONSTER_SAY)
                return true
            elseif targetItem.message then
                player:sendTextMessage(MESSAGE_EVENT_ADVANCE, targetItem.message.text)
            else
                return false
            end
        else
            if targetItem.targetId then
                item:transform(targetItem.id)
                item:decay()
                --player:sendSupplyUsed(target)
                target:remove(1)
            else
                if targetItem.usesStorage then
                    local vocationId = player:getVocation():getDemotion():getId()
                    local storage = storages[itemId] and storages[itemId][targetId] and storages[itemId][targetId][vocationId]
                    if not storage then
                        player:sendCancelMessage(RETURNVALUE_NOTPOSSIBLE)
                        return true
                    end

                    local storageValue = player:getStorageValue(storage.key)
                    if not storageValue or storageValue <= 0 then
                        player:sendCancelMessage(RETURNVALUE_NOTPOSSIBLE)
                        return true
                    end

                    local transform = targetItem.id and targetItem.id[storageValue]
                    if not transform then
                        player:sendCancelMessage(RETURNVALUE_NOTPOSSIBLE)
                        return true
                    end
                    target:transform(transform.id)
                    applyEnchantmentLevel(target, targetItem, transform.id)
                else
                    target:transform(targetItem.id)
                    applyEnchantmentLevel(target, targetItem, targetItem.id)
                end
   
                if target:hasAttribute(ITEM_ATTRIBUTE_DURATION) then
                    target:decay()
                end
   
                if target:hasAttribute(ITEM_ATTRIBUTE_CHARGES) then
                    target:setAttribute(ITEM_ATTRIBUTE_CHARGES, config.equipment.charges)
                end
                --player:sendSupplyUsed(item)
                item:remove(1)
            end
        end
        player:getPosition():sendMagicEffect(targetItem.effects and (isInSlot and targetItem.effects.failure or targetItem.effects.success) or config.equipment.effect)
    end
    return true
end