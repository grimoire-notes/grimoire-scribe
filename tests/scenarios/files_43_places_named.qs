# settings: status_mode=1
# settings: files_card=1
# storage: 1
# generate: tests/files_places_named_tree.py
# env: GRIMOIRE_PLACE_DOCUMENTS=Document GRIMOIRE_PLACE_DOWNLOADS=Download GRIMOIRE_PLACE_FONTS= GRIMOIRE_STORAGE_SYSTEM=Stock
# Files on a storage that names its places otherwise (issue #278, off the Kindle: a platform whose data root sits
# inside a larger storage): Documents and Downloads open the folders their variables name, with the same labels and
# runner names; there is no Kindle fonts place, set empty, though a fonts folder is there; the folders of the
# Kindle's names are folders like any other, in Go to by their names, and Document is in Go to once, as the
# Documents place. The storage's own system folder (Stock) is listed nowhere and refused.
# rust-only
wait 300
tap library files-card
wait 300
expect files.mode is place
expect files.place is root
expect files.rows is 8
expect files.row.1 is Import
expect files.row.2 is Documents
expect files.sub.2 is Document
expect files.row.3 is Downloads
expect files.sub.3 is Download
expect files.row.4 is Templates
expect files.row.5 is Photos
expect files.row.6 is Fonts
expect files.row.7 is Trash
expect files.row.8 is All storage
expect files.item.Kindle_fonts is
shot 01-root
tap library files.item.documents
wait 300
expect files.place is Document
expect files.crumbs is Files > Documents
expect files.item.Books is Books
tap library files.item.Books
wait 300
expect files.place is Document/Books
expect files.crumbs is Files > Documents > Books
expect files.item.Book.pdf is Book.pdf
tap library files.crumb.2
wait 300
expect files.place is Document
tap library files.crumb.2
wait 300
expect files.card is goto
expect files.card.rows is Home: Notebooks|Import|Documents|Downloads|Templates|Photos|Fonts|Trash|All storage|documents|fonts|Notes|Set as home|Cancel
shot 02-go-to
tap library files.goto.9
wait 300
expect files.card is
expect files.place is documents
expect files.crumbs is Files > All storage > documents
tap library files.crumb.1
wait 300
expect files.place is root
tap library files.item.All_storage
wait 300
expect files.place is
expect files.item.Document is Document
expect files.item.Download is Download
expect files.item.documents is documents
expect files.item.fonts is fonts
expect files.item.Notes is Notes
expect files.item.Stock is
shot 03-all-storage
tap library files.back
wait 200
tap library files.item.Downloads
wait 300
expect files.place is Download
expect files.crumbs is Files > Downloads
expect files.item.Daily.pdf is Daily.pdf
tap library files.back
wait 200
tap library files.back
wait 300
expect files.open is 0
quit
