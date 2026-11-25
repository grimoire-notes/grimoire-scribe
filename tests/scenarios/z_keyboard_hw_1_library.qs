# settings: shortcuts_enabled=1
# settings: mirror_pdf=0
# rust-only
# A paired keyboard types into the Library's keyboards (issue #266): its letters (Shift for the capitals), digits,
# punctuation and Space type at the caret as the keys do, Backspace deletes, Enter is Done and Esc is Cancel, and a
# name keyboard refuses what names refuse when it is done. With no keyboard up a letter is the shortcut it was.
# Keys by Linux evdev code, the US layout: T 20, R 19, I 23, P 25, O 24, M 50, E 18, A 30, B 48, Z 44, the digits
# 2 to 11 (1 to 9, 0), / 53, Space 57, Backspace 14, Enter 28, Esc 1, End 107; mods 2 is Shift, 1 Ctrl. (On the
# Kindle a keyboard is read only while Settings > Shortcuts is on, as here.)
# (the clock corner held still: its minute's refresh would land among the counted ones)
clock 1700000000
wait 3600
# No keyboard up: O is Settings, as before, and Esc closes it
key 24 0
wait 100
expect library.sheet is settings
key 1 0
wait 100
expect library.sheet is none
expect library.items is 0
# New notebook (Ctrl+N), and its name's keyboard: the name it offers is selected, so the first key replaces it
key 49 1
wait 100
expect library.sheet is new
tap library new.rename
wait 100
expect library.sheet is keyboard
expect display.presents not -1
expect keyboard.draws not -1
key 20 2
expect library.caret is T|
# a key's character costs what an on-screen key's does, without the key: the text line alone, the keyboard drawn once
expect display.presents is fast 50 1510 1760 120
expect keyboard.draws is 1
key 19 0
key 23 0
key 25 0
key 57 0
key 3 0
key 11 0
key 3 0
key 7 0
expect library.caret is Trip 2026|
# Enter is Done: New notebook again, with the name; Enter there is Create, as before
key 28 0
wait 100
expect library.sheet is new
key 28 0
wait 300
expect screen is editor
key 1 0
wait 300
expect screen is library
expect library.items is 1
expect library.card.1.name is Trip 2026
# Rename: the name selected; End lets it go, Backspace takes four characters, Rome is typed, Esc cancels it all
tap library card-menu.1
wait 200
tap library more.rename
wait 200
expect library.sheet is keyboard
expect library.caret is [|Trip 2026]
key 107 0
expect library.caret is Trip 2026|
key 14 0
key 14 0
key 14 0
key 14 0
expect library.caret is Trip |
key 19 2
key 24 0
key 50 0
key 18 0
expect library.caret is Trip Rome|
key 1 0
wait 200
expect library.sheet is none
expect library.card.1.name is Trip 2026
# The same again, and Enter renames it
tap library card-menu.1
wait 200
tap library more.rename
wait 200
key 107 0
key 14 0
key 14 0
key 14 0
key 14 0
key 19 2
key 24 0
key 50 0
key 18 0
key 28 0
wait 300
expect library.sheet is none
expect library.card.1.name is Trip Rome
# A character a name refuses: a folder's name with a slash (Ctrl+F, New folder) is typed, and Done says no
key 33 1
wait 100
expect library.sheet is keyboard
key 30 0
key 53 0
key 48 0
expect library.caret is a/b|
key 28 0
wait 200
expect toast is Couldn't create folder. Use a shorter name without a slash.
expect library.sheet is none
expect library.folders is
# The keyboard's length: 95 bytes and its end; the 96th key types nothing, a Backspace makes room for it
key 33 1
wait 100
expect library.sheet is keyboard
key 2 0
key 3 0
key 4 0
key 5 0
key 6 0
key 7 0
key 8 0
key 9 0
key 10 0
key 11 0
key 2 0
key 3 0
key 4 0
key 5 0
key 6 0
key 7 0
key 8 0
key 9 0
key 10 0
key 11 0
key 2 0
key 3 0
key 4 0
key 5 0
key 6 0
key 7 0
key 8 0
key 9 0
key 10 0
key 11 0
key 2 0
key 3 0
key 4 0
key 5 0
key 6 0
key 7 0
key 8 0
key 9 0
key 10 0
key 11 0
key 2 0
key 3 0
key 4 0
key 5 0
key 6 0
key 7 0
key 8 0
key 9 0
key 10 0
key 11 0
key 2 0
key 3 0
key 4 0
key 5 0
key 6 0
key 7 0
key 8 0
key 9 0
key 10 0
key 11 0
key 2 0
key 3 0
key 4 0
key 5 0
key 6 0
key 7 0
key 8 0
key 9 0
key 10 0
key 11 0
key 2 0
key 3 0
key 4 0
key 5 0
key 6 0
key 7 0
key 8 0
key 9 0
key 10 0
key 11 0
key 2 0
key 3 0
key 4 0
key 5 0
key 6 0
key 7 0
key 8 0
key 9 0
key 10 0
key 11 0
key 2 0
key 3 0
key 4 0
key 5 0
key 6 0
expect library.caret is 12345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345|
wait 1200
expect display.presents not -1
key 7 0
expect library.caret is 12345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345|
expect display.presents is
key 14 0
key 7 0
expect library.caret is 12345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012346|
key 1 0
wait 200
expect library.sheet is none
expect library.folders is
# The home's search: its keyboard filters as it is typed; Enter keeps the search, Esc on the Library clears it
tap library search
wait 100
expect library.sheet is keyboard
key 19 0
key 24 0
key 50 0
key 18 0
wait 100
expect library.caret is rome|
expect library.items is 1
key 14 0
key 14 0
key 14 0
key 14 0
key 44 0
key 44 0
wait 100
expect library.caret is zz|
expect library.items is 0
key 28 0
wait 200
expect library.sheet is none
expect library.items is 0
key 1 0
wait 200
expect library.items is 1
quit
