-- Door and key item ids. Loaded at startup by `doors.rs`.
-- Pack surface: former `data/global.lua` arrays + `scripts/actions/other/doors.lua`.
-- Add later-era ids as extra rows.

return {
	schema = 1,
	keys = {
		2086, 2087, 2088, 2089, 2090, 2091, 2092,
	},
	open = {
		1211, 1214, 1233, 1236, 1251, 1254, 3546, 3537, 4915, 4918, 1635,
	},
	closed = {
		1210, 1213, 1232, 1235, 1250, 1253, 3536, 3545, 4914, 4917,
	},
	locked = {
		1209, 1212, 1231, 1234, 1249, 1252, 3535, 3544, 4913, 4916,
	},
	openExtra = {
		1540, 1542,
	},
	closedExtra = {
		1539, 1541,
	},
	openHouse = {
		1220, 1222, 1238, 1240, 3539, 3548,
	},
	closedHouse = {
		1219, 1221, 1237, 1239, 3538, 3547,
	},
	openQuest = {
		1224, 1226, 1242, 1244, 1256, 1258, 3543, 3552,
	},
	closedQuest = {
		1223, 1225, 1241, 1243, 1255, 1257, 3542, 3551,
	},
	openLevel = {
		1228, 1230, 1246, 1248, 1260, 1262, 3541, 3550,
	},
	closedLevel = {
		1227, 1229, 1245, 1247, 1259, 1261, 3540, 3549,
	},
}
