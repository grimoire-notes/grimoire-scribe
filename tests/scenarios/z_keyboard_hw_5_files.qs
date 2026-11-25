# settings: status_mode=1
# settings: files_card=1
# settings: shortcuts_enabled=1
# storage: 1
# generate: tests/files_trash_tree.py
# rust-only
# A paired keyboard types into Files' Rename (issue #266), at the caret, which opens before the extension: a
# character a name cannot hold (Shift with ; is a colon) is typed as on the keys, and Done refuses it and keeps
# the keyboard up with it; a Backspace takes it away, and a good name typed and Enter renames the file. Keys by
# Linux evdev code, the US layout (mods 2 is Shift): ; 39, D 32, O 24, T 20, S 31, Backspace 14, Enter 28,
# Home 102, End 107.
wait 3600
tap library files-card
wait 300
tap library files.item.Templates
wait 300
expect files.place is grimoire/templates
expect files.order is Grid.png|Lines.png
tap library files.item.Grid.png
wait 300
tap library files.card.rename
wait 300
expect files.keyboard is 1
expect files.text is Grid.png
key 39 2
expect files.text is Grid:.png
key 28 0
wait 200
expect toast is Name can't contain :
expect files.keyboard is 1
expect files.text is Grid:.png
key 14 0
expect files.text is Grid.png
key 31 0
expect files.text is Grids.png
key 28 0
wait 300
expect files.keyboard is 0
expect storage.names.grimoire/templates is Grids.png|Lines.png|README.txt
quit
