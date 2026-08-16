# Floor-down crash — bug0000013 `rz=-1` on non-adjacent NotifyGo

**Status:** done. Lesson 361.

Non-adjacent `send_notify_go` is `SendFullScreen` (`0x64`) only — no leading `0x6D`. Adjacent hole/stairs still `0x6C`/`0x6D` + floors (lesson 277).
