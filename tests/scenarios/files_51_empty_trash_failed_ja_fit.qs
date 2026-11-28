# settings: status_mode=1
# settings: files_card=1
# storage: 1
# generate: tests/files_trash_tree.py
# env: GRIMOIRE_EMPTY_TEST_BUSY=1
# env: GRIMOIRE_LANG=ja
# env: GRIMOIRE_CHECK_FIT=1
# Empty trash that cannot work on the trash (2.3, issue #330) with the language set to ja: the toast that says the
# trash was not emptied, checked for fit (English there until it is translated). files_49_empty_trash_failed runs it
# in English with its other checks.
# rust-only
wait 3600
tap library files-card
wait 300
waitfor files.sub.Trash has 7 KB
tap library files.item.Trash
wait 300
waitfor files.empty is 1
tap library files.empty
wait 500
expect files.card is empty
tap library files.card.empty
waitfor toast has trash
waitfor files.ready is 1
expect files.rows is 2
shot 01-not-emptied
quit
