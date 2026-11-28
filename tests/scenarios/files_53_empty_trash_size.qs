# settings: status_mode=1
# settings: files_card=1
# settings: mirror_pdf=0
# storage: 1
# generate: tests/files_empty_trash_tree.py
# env: GRIMOIRE_CHECK_FIT=1
# Empty trash says one size from the row to the toast (2.3, issue #330): a photo of exactly 3,072 bytes deleted in
# Files is a `files-` entry with its record beside it. The Trash place's row says 3 KB, the question "1 entry (3 KB)
# will be deleted for good.", and the toast "Deleted 1 entry, freed 3 KB.": the record's bytes are counted nowhere.
# rust-only
wait 3600
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
tap library files.crumb.1
wait 300
tap library files.item.Trash
wait 300
waitfor files.empty is 1
expect files.sub.1 has 3 KB,
tap library files.empty
wait 500
expect files.card.text is 1 entry (3 KB) will be deleted for good.
tap library files.card.empty
waitfor toast has Deleted 1 entry
expect toast is Deleted 1 entry, freed 3 KB.
waitfor files.ready is 1
expect files.rows is 0
quit
