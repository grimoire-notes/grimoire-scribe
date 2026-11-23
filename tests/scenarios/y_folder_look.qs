# settings: status_mode=1
# storage: 1
# generate: tests/folder_look_tree.py
# A folder's icon and picture (2.2, issue #230): School's menu opens its icon sheet, where one of the icons is
# picked and saved; the picture sheet lists the pictures of grimoire/photos/ (From a file... opens the picker over
# it, and Back comes back), and a picture picked takes its place on the folder's card. Done closes; the look is
# read back when the sheet opens again; No picture leaves the icon, and the plain folder icon takes the look out
# of folder-looks.ini altogether.
# rust-only
wait 300
expect library.items is 1
expect library.card.1 is folder
expect library.card.1.name is School
tap library folder-menu.School
wait 200
expect library.sheet is folder-more
tap library fmore.look
wait 200
expect library.sheet is folder-look
expect library.look.folder is School
expect library.look.icon is
expect library.look.picture is
shot 01-icon-sheet
tap library folder-icon.castle
wait 200
expect library.look.icon is castle
tap library folder-look.done
wait 300
expect library.sheet is none
# the look is kept: the sheet opens on it
tap library folder-menu.School
wait 200
tap library fmore.look
wait 200
expect library.look.icon is castle
# the pictures' sheet: From a file... opens the picker over it, and Back comes back to it
tap library folder-picture.open
wait 200
expect library.sheet is folder-picture
expect library.look.picture is
shot 02-picture-sheet
tap library folder-picture.file
wait 300
expect files.open is 1
expect files.mode is pick
tap library files.back
wait 300
expect files.open is 0
expect library.sheet is folder-picture
# its photo's preview is made by the background job; the open sheet picks it up
waitbg
wait 1600
expect library.ui_has.folder-picture.cover.jpg is 1
tap library folder-picture.cover.jpg
wait 300
expect library.sheet is folder-look
expect library.look.picture is cover.jpg
shot 03-picture-chosen
tap library folder-look.done
wait 300
expect library.sheet is none
expect library.card.1 is picture
# the icon stays under the picture; No picture brings it back
tap library folder-menu.School
wait 200
tap library fmore.look
wait 200
tap library folder-picture.none
wait 300
expect library.look.picture is
expect library.look.icon is castle
tap library folder-look.done
wait 300
expect library.card.1 is folder
# the plain folder icon takes the look out of folder-looks.ini
tap library folder-menu.School
wait 200
tap library fmore.look
wait 200
tap library folder-icon.folder
wait 200
expect library.look.icon is
tap library folder-look.done
wait 300
expect library.card.1 is folder
quit
