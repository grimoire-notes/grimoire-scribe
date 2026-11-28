# settings: status_mode=1
# settings: files_card=1
# settings: mirror_pdf=0
# storage: 1
# generate: tests/files_empty_trash_tree.py
# env: GRIMOIRE_CHECK_FIT=1
# Empty trash (2.3, issue #330), every text checked for fit: a notebook deleted from the Library (Beta), a page from
# the Pages view (Alpha's second), and a file in Files (beach.jpg); then Files > Trash lists the three, and its path bar
# has Empty trash once the trash's size is known (shot 01). Its tap asks first, in the question's sheet ("Empty the
# trash?", how many entries and how much, Empty trash and Cancel: shot 02). Its Empty trash deletes the three for good
# in the background job: the toast says how many and the space freed, the place says the trash is empty and offers no
# Empty trash, the trash folder holds nothing, and the Library is as the deletes left it (Alpha with one page, Gamma in
# School, the photo gone from Photos).
# rust-only
wait 3600
expect library.items is 4
# Beta, from the Library
tap library notebook-menu.20261101-000000-0002
wait 200
expect library.sheet is more
tap library more.delete
wait 500
expect library.sheet is delete
tap library delete.yes
wait 300
expect toast is Moved to grimoire/trash
expect library.items is 3
# Alpha's second page, from the Pages view (tile 2's centre upright is 930 551)
tap library notebook.20261101-000000-0001
wait 600
expect screen is editor
expect editor.pages is 2
pentap panel pages
wait 400
expect editor.modal is pages
touch down 930 551
wait 700
touch up 930 551
expect editor.pages.context is 2
tap panel pages.delete
wait 400
expect editor.pages is 1
tap 125 170
wait 300
pentap panel library
wait 600
expect screen is library
# the photo, from Files
tap library files-card
wait 300
tap library files.item.Photos
wait 300
tap library files.item.beach.jpg
wait 300
tap library files.card.trash
wait 500
tap library files.card.delete
wait 300
expect toast is Moved to grimoire/trash
expect storage.names.grimoire/notebooks is 20261101-000000-0001|20261101-000000-0003
# Files > Trash: the three, newest first, and Empty trash once the trash's size is known
tap library files.crumb.1
wait 300
tap library files.item.Trash
wait 300
expect files.place is grimoire/trash
expect files.rows is 3
waitfor files.row.1 is beach.jpg
waitfor files.row.3 is Beta
waitfor files.empty is 1
expect files.kind.2 is trashed
waitfor toast is
shot 01-trash-empty-trash
# it asks first
tap library files.empty
wait 500
expect files.card is empty
expect files.card.details is Empty the trash?
expect files.card.text has 3 entries (
expect files.card.text has ) will be deleted for good.
expect files.card.rows is Empty trash|Cancel
expect storage.names.grimoire/trash not
shot 02-empty-trash-question
# and its Empty trash deletes them for good
tap library files.card.empty
waitfor toast has Deleted 3 entries, freed
expect toast has KB.
waitfor files.ready is 1
expect files.card is
expect files.rows is 0
expect files.none is The trash is empty.
expect files.empty is 0
expect storage.names.grimoire/trash is
shot 03-emptied
# the Library and the folders as the deletes left them
expect storage.names.grimoire/notebooks is 20261101-000000-0001|20261101-000000-0003
expect storage.exists.grimoire/notebooks/20261101-000000-0001/p1.qpg is 1
expect storage.exists.grimoire/photos/beach.jpg is 0
expect storage.size.grimoire/templates/Grid.png is 1024
tap library files.crumb.1
wait 300
waitfor files.sub.Trash is grimoire/trash
tap library files.back
wait 300
expect files.open is 0
expect library.items is 3
expect library.card.2.name is Alpha
quit
