# rust-only
# root: z_bar_0_make
# A paired keyboard types into the notebook's keyboards (issue #266): Find, the page's name and New page...'s.
# Its letters, digits and Space type as the keys do, Backspace deletes, Enter is Done and Esc is Cancel; with no
# keyboard up a letter is the shortcut it was (this notebook's shortcuts.ini, tests/fixtures/index-keys: G Find,
# N the page's name, L a new link, X close-menu). Keys by Linux evdev code, the US layout (mods 2 is Shift): A 30,
# R 19, E 18, X 45, B 48, C 46, H 35, I 23, D 32, S 31, G 34, N 49, L 38, the digit 5 is 6, Space 57, Backspace 14,
# Enter 28, Esc 1, End 107.
wait 3600
tap library card.1
wait 300
expect editor.page is 1
# No keyboard up: G is Find
key 34 0
wait 300
expect editor.find is go
# are: Arena alone; a Backspace, ar: both, Arbol first; Enter goes to the first row's page
key 30 0
key 19 0
key 18 0
expect editor.caret is are|
expect editor.find.rows is 1
expect editor.find.1 is Arena
key 14 0
expect editor.caret is ar|
expect editor.find.rows is 2
expect editor.find.1 is Árbol
key 28 0
wait 300
expect editor.find is 0
expect editor.page is 3
# A digit: Go to page 5, and Enter goes there
key 34 0
wait 300
key 6 0
expect editor.find.1 is Go to page 5
key 28 0
wait 300
expect editor.find is 0
expect editor.page is 5
# X, close-menu without a keyboard, types x in one; Esc is Cancel, the page as it was
key 34 0
wait 300
key 45 0
expect editor.caret is x|
expect editor.find is go
key 1 0
wait 300
expect editor.find is 0
expect editor.page is 5
# N: the page's name (Arena), typed over: End, five Backspaces, Beach with Shift, Enter
key 49 0
wait 300
expect editor.keyboard is Page name
expect editor.caret is Arena|
key 107 0
key 14 0
key 14 0
key 14 0
key 14 0
key 14 0
key 48 2
key 18 0
key 30 0
key 46 0
key 35 0
expect editor.caret is Beach|
key 28 0
wait 300
expect editor.keyboard is
expect editor.page_name is Beach
# New page... in Link to... (L, then the link's area drawn): its keyboard names the page at the end
key 38 0
expect editor.link_mode is 1
pen line 1000 600 1500 800 30 0.6 0.6
expect editor.linkto.1 is new
tap panel linkto.chip.1
wait 100
expect editor.keyboard is Name the new page
key 23 2
key 32 0
key 18 0
key 30 0
key 31 0
key 57 0
key 6 0
expect editor.caret is Ideas 5|
key 28 0
wait 100
expect editor.keyboard is
expect editor.pages is 6
expect toast is Linked to new page 6.
quit
