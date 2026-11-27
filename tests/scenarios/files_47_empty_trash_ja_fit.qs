# settings: status_mode=1
# settings: files_card=1
# storage: 1
# generate: tests/files_trash_tree.py
# env: GRIMOIRE_LANG=ja
# env: GRIMOIRE_CHECK_FIT=1
# Empty trash (2.3, issue #330) with the language set to ja, every text checked for fit (its new texts are English
# there until they are translated): the Trash place with Empty trash on its path bar, its question, the job's staying
# notice and toast, and the place that says the trash is empty. files_44_empty_trash runs it in English.
# rust-only
wait 3600
tap library files-card
wait 300
waitfor files.sub.Trash has 7 KB
tap library files.item.Trash
wait 300
expect files.rows is 2
waitfor files.empty is 1
shot 01-trash
tap library files.empty
wait 500
expect files.card is empty
shot 02-question
tap library files.card.empty
waitfor toast has 2
waitfor files.ready is 1
expect files.rows is 0
expect files.none not
expect files.empty is 0
expect storage.names.grimoire/trash is
shot 03-emptied
quit
