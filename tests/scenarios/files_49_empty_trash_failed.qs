# settings: status_mode=1
# settings: files_card=1
# storage: 1
# generate: tests/files_trash_tree.py
# env: GRIMOIRE_EMPTY_TEST_BUSY=1
# env: GRIMOIRE_CHECK_FIT=1
# Empty trash that cannot work on the trash (2.3, issue #330): another Empty trash holds the trash's staging folder
# (as the detached child of a session before can; GRIMOIRE_EMPTY_TEST_BUSY holds its lock), so the job deletes
# nothing and its run ends 1. The toast says the trash was not emptied, never "Deleted 0 entries", its text checked
# for fit; both entries are still in the trash and listed, Empty trash is offered again, and the notebook's card
# offers Restore again. files_50 to files_52 check the toast's fit in es, ja and pl.
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
tap library files.card.empty
waitfor toast has Couldn't
expect toast is Couldn't empty the trash.
expect notice is
waitfor files.ready is 1
expect files.rows is 2
waitfor files.empty is 1
waitfor files.row.1 is Beta
shot 01-not-emptied
tap library files.row.1
wait 300
expect files.card is trashed
expect files.card.rows is Restore|Cancel
quit
