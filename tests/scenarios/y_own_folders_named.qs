# settings: status_mode=1
# The texts that name one of Grimoire's own folders name it as found from the storage root (issue #274,
# pdfimport::own_folder_shown), not as the Kindle's "grimoire/...". This run sets no storage root (no "# storage:"
# line), as a platform kept in another crate may not: the storage is then the folder around the data root, which
# the runner makes as build/scen/y_own_folders_named, so every such text names y_own_folders_named/<folder>.
# A notebook's menu (Save PDF to y_own_folders_named/PDFs now), its Delete question (Delete (moves it to
# y_own_folders_named/trash)) and the toast after it; Settings > System's Mirror PDFs row. On the Kindle, and in a
# "# storage: 1" run, the same texts say grimoire/PDFs and grimoire/trash (files_10_restore_notebook).
# rust-only
wait 200
tap library add
tap library add.new-notebook
wait 100
expect library.sheet is new
tap 1564 2254
wait 300
expect screen is editor
pentap panel library
wait 400
expect screen is library
expect library.items is 1
tap 396 1197
wait 200
expect library.sheet is more
shot 01-more-menu
tap library more.delete
wait 500
expect library.sheet is delete
shot 02-delete-asked
tap library delete.yes
wait 300
expect library.sheet is none
expect library.items is 0
expect toast is Moved to y_own_folders_named/trash
shot 03-moved
tap library settings
wait 50
expect library.sheet is settings
tap library settings.cat.5
wait 50
shot 04-settings-system
tap library done
wait 50
expect library.sheet is none
quit
