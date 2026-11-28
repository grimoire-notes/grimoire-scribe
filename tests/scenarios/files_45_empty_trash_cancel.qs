# settings: status_mode=1
# settings: files_card=1
# settings: theme=classic
# storage: 1
# generate: tests/files_trash_tree.py
# env: GRIMOIRE_CHECK_FIT=1
# Empty trash's question answered no (2.3, issue #330), in Classic, every text checked for fit: the Trash place lists
# its two entries (a notebook of 5 KB, a page of 2 KB) with Empty trash on its path bar; its question says "2 entries
# (7 KB) will be deleted for good."; Cancel deletes nothing (the question gone, no job, both entries in the trash and
# listed, Empty trash still offered, the notebook's card still offering Restore); asked again, a tap outside the
# question does the same.
# rust-only
wait 3600
tap library files-card
wait 300
waitfor files.sub.Trash is grimoire/trash, 7 KB
tap library files.item.Trash
wait 300
expect files.rows is 2
waitfor files.empty is 1
tap library files.empty
wait 500
expect files.card is empty
expect files.card.details is Empty the trash?
expect files.card.text is 2 entries (7 KB) will be deleted for good.
expect files.card.rows is Empty trash|Cancel
shot 01-question-classic
# Cancel: nothing deleted, nothing asked of a job
tap library files.card.cancel
wait 300
expect files.card is
expect bg.busy is 0
expect files.rows is 2
expect files.empty is 1
expect storage.names.grimoire/trash is 20261101-000000-0001-p3-1789995000|20261101-000000-0004-1789996000
# asked again, and a tap outside the question (the header): the same
tap library files.empty
wait 500
expect files.card is empty
tap 1700 300
wait 300
expect files.card is
expect bg.busy is 0
expect storage.names.grimoire/trash is 20261101-000000-0001-p3-1789995000|20261101-000000-0004-1789996000
waitfor files.row.1 is Beta
tap library files.row.1
wait 300
expect files.card is trashed
expect files.card.rows is Restore|Cancel
shot 02-after-cancel
quit
