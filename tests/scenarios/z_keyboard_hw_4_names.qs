# rust-only
# root: z_bar_0_make
# A paired keyboard types into the notebook's other keyboards (issue #266): a link card's Rename, the Graph's
# Rename and Move to...'s number pad, which takes a digit alone (a letter types nothing there, as on its keys).
# Keys by Linux evdev code, the US layout (mods 2 is Shift): C 46, A 30, P 25, X 45, the digit 5 is 6, Backspace
# 14, Enter 28, Esc 1.
wait 3600
tap library card.1
wait 300
expect editor.page is 1
# A link held: its card; Rename opens the keyboard with the target's name, Arbol; five Backspaces, Cap, Enter
touch down 600 1100
wait 800
expect editor.link_card is 1
touch up 600 1100
tap 679 1428
wait 100
expect editor.keyboard is Page name
expect editor.caret is Árbol|
key 14 0
key 14 0
key 14 0
key 14 0
key 14 0
key 46 2
key 30 0
key 25 0
expect editor.caret is Cap|
key 28 0
wait 100
expect editor.keyboard is
expect editor.link_card.title is Cap
tap 1200 1900
expect editor.link_card is 0
# The Graph's Rename of page 2: a, Enter
tap panel pages
tap panel pages.graph
tap map-node 2
expect editor.map.selected is 2
tap panel map.rename
expect editor.keyboard is Page name
expect editor.text is
key 30 0
expect editor.text is a
key 28 0
expect editor.keyboard is
expect editor.index_rows is 3
expect editor.map is graph
tap 100 170
expect editor.map is 0
# Move to... (page 2's card in the Pages view): its number pad takes 5 and not x; Enter moves the page
tap panel pages
waitbg
wait 200
expect editor.modal is pages
touch down 930 551
wait 700
touch up 930 551
expect editor.pages.context is 2
tap panel pages.move-to
wait 100
expect editor.keyboard is Move before page
key 45 0
expect editor.text is
key 6 0
expect editor.text is 5
key 28 0
wait 200
expect editor.keyboard is
expect editor.modal is pages
expect editor.page_ids is 1,3,4,2,5
quit
