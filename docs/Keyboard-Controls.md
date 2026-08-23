# Physical Keyboard Controls

The standalone frontend and the libretro core use the same physical keyboard
mapping. Keys operate the Wenquxing hardware keypad, so two PC keys may target
the same device key when the real key cap carries both legends.

## Common Keys

| PC key | Wenquxing key |
|--------|---------------|
| A-Z | Matching letter key |
| 1 / B | B / 1 |
| 2 / N | N / 2 |
| 3 / M | M / 3 |
| 4 / G | G / 4 |
| 5 / H | H / 5 |
| 6 / J | J / 6 |
| 7 / T | T / 7 |
| 8 / Y | Y / 8 |
| 9 / U | U / 9 |
| 0 | Symbol / 0 / Continue |
| Arrow keys | Up / Down / Left / Right |
| Enter | Enter / Confirm |
| Escape | Escape / Back |
| Space or `=` | Space |
| Backspace or F2 | F2 / Delete |
| PageUp or `,` | Page Up |
| PageDown or `/` | Page Down |
| `[` | Help |
| `]` | Wenquxing Shift / Language mode |
| `\` | Input method |
| `.` | Decimal point / Dot |
| F1-F4 | F1-F4 |
| F12 or Delete | Power |

The PC Shift key remains a host modifier. Use `]` for the Wenquxing Shift key.
Close the standalone window or use the operating system's standard close
shortcut to exit; Escape is reserved for the emulated device.

## Model Hotkeys

| PC key | NC1020 / PC1000 / CC800 / NC2000 | NC3000 |
|--------|------------------------------------|--------|
| F5 | Dictionary | Game |
| F6 | Card / Contacts | Calculator |
| F7 | Calculator | Time |
| F8 | Memo / Schedule | Unmapped |
| F9 | Data | Dictionary |
| F10 | Time | Lexicon |
| F11 | Network | Study |

The exact labels and secondary functions are visible on the standalone device
skin and depend on the selected model and firmware.

## RetroArch Keyboard Focus

RetroArch reserves Escape and several function keys for frontend commands. Turn
on Game Focus before using the physical keyboard; its default toggle is usually
Scroll Lock and can be changed in RetroArch's input settings. Alternatively,
configure a Hotkey Enable button so frontend hotkeys activate only while that
button is held.

RetroArch's Quick Menu > Controls remaps the RetroPad. It does not replace this
physical keyboard mapping. Physical keyboard and RetroPad input can be used at
the same time.
