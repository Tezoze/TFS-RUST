-- Door and key item ids. Loaded at startup by `doors.rs`.
-- Pack surface: former `data/global.lua` arrays + `scripts/actions/other/doors.lua`.
-- Add later-era ids as extra rows.

return {
	schema = 1,
	keys = {
		2967, 2968, 2969, 2970, 2971, 2972, 2973,
	},
	open = {
		1630, 1633, 1652, 1655, 1670, 1673, 1693, 1684, 4914, 4911, 2335,
	},
	closed = {
		1629, 1632, 1651, 1654, 1669, 1672, 1683, 1692, 5006, 5007,
	},
	locked = {
		1628, 1631, 1650, 1653, 1668, 1671, 1682, 1691, 4913, 4912,
	},
	openExtra = {
		2178, 2180,
	},
	closedExtra = {
		2177, 2179,
	},
	openHouse = {
		1639, 1641, 1657, 1659, 1686, 1695,
	},
	closedHouse = {
		1638, 1640, 1656, 1658, 1685, 1694,
	},
	openQuest = {
		1643, 1645, 1661, 1663, 1675, 1677, 1690, 1699,
	},
	closedQuest = {
		1642, 1644, 1660, 1662, 1674, 1676, 1689, 1698,
	},
	openLevel = {
		1647, 1649, 1665, 1667, 1679, 1681, 1688, 1697,
	},
	closedLevel = {
		1646, 1648, 1664, 1666, 1678, 1680, 1687, 1696,
	},
}
