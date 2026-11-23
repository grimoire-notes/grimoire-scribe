# settings: status_mode=1
# settings: status_on=1
# settings: status_in_notebooks=1
# env: TZ=UTC
# env: GRIMOIRE_SIM_LIGHT_SENSOR=1
# rust-only
# Quick settings is the app's: a one-finger pull from the top works on the home screen, over an app,
# and in a notebook. It has brightness and warmth, Settings, rotation lock and auto brightness.
# The status bar has no background in a notebook, a tap hides it for good, and the Status bar page
# has Show status bar and Show status bar in notebooks.
wait 3800
clock 1700000000
battery 87
wait 100
expect screen is library
expect status.visible is 1
expect quick is 0
# On the home screen: a one-finger pull from the bar opens the panel.
touch down 930 40
touch move 930 120
wait 150
expect quick is 1
touch up 930 120
wait 150
expect quick is 1
# A compact sheet, with margins: the title and buttons are inside it, not a big empty chin below.
expect quick.rect is 0 78 1860 556
shot 00-home-quick-settings
tap panel quick.close
wait 200
expect quick is 0
# A tap on the bar hides it; the state is saved (settings.ini status_on).
tap 930 40
wait 300
expect settings.status_on is 0
expect status.visible is 0
expect status.zone is 0 0 0 0
shot 01-status-hidden
# The top strip remains the system quick settings gesture even with the bar gone.
touch down 930 40
touch move 930 140
wait 150
expect quick is 1
touch up 930 140
wait 100
expect quick.rect is 0 0 1860 556
tap panel quick.close
wait 200
# Show status bar is a row on the Status bar page, so a hidden bar can always come back.
tap library settings
wait 300
tap library settings.cat.2
wait 100
tap library status-bar
wait 200
expect editor.settings.sheet is display
expect status.visible is 0
shot 02-status-settings
tap library display.show
wait 200
expect settings.status_on is 1
expect status.visible is 1
tap library display.notebooks
wait 200
expect settings.status_in_notebooks is 0
tap library display.notebooks
wait 200
expect settings.status_in_notebooks is 1
tap library display.done
wait 300
expect editor.settings.sheet is none
expect status.visible is 1
shot 02-status-back
# Into a notebook. Its bar is the one shared bar, drawn transparent over the page (nothing reserved).
tap library add
wait 100
tap library add.new-notebook
wait 100
tap 510 460
tap 1564 2254
wait 300
expect screen is editor
expect status.reserved is 0
expect status.visible is 1
expect quick is 0
# The pull opens the same panel over the notebook.
touch down 930 40
touch move 930 120
wait 150
expect quick is 1
touch up 930 120
wait 150
expect quick is 1
expect quick.auto is 0
expect quick.brightness is 12
expect quick.warmth is 8
shot 03-notebook-quick-settings
# The brightness slider follows a tap along its row; the front light is set as it moves.
tap panel quick.brightness
wait 150
expect quick.brightness not 12
expect quick.warmth is 8
tap panel quick.warmth
wait 150
expect quick.warmth not 8
expect settings.rotation_lock is 0
tap panel quick.rotate
wait 150
expect settings.rotation_lock not 0
tap panel quick.auto
wait 150
expect quick.auto is 1
shot 04-quick-settings-changed
tap panel quick.close
wait 200
expect quick is 0
# The Settings button saves the page and opens Settings over the notebook, on the Status bar page.
touch down 930 40
touch move 930 140
wait 150
expect quick is 1
touch up 930 140
wait 100
tap panel quick.settings
wait 300
expect quick is 0
expect editor.modal is settings
tap library settings.cat.2
wait 100
tap library status-bar
wait 200
expect editor.settings.sheet is display
# Show status bar in notebooks off: the notebook has no bar, but the home screen keeps it.
tap library display.notebooks
wait 200
expect settings.status_in_notebooks is 0
tap library display.done
wait 300
expect editor.modal is none
expect status.visible is 0
expect settings.status_on is 1
# A tap where the bar would be is not a bar tap: it is not shown in notebooks.
tap 930 40
wait 200
expect settings.status_on is 1
expect quick is 0
# The top pull still opens quick settings, and Show status bar in notebooks can turn it back on.
touch down 930 40
touch move 930 140
wait 150
expect quick is 1
touch up 930 140
wait 100
tap panel quick.settings
wait 300
tap library status-bar
wait 200
tap library display.notebooks
wait 200
expect settings.status_in_notebooks is 1
tap library display.done
wait 300
expect editor.modal is none
expect status.visible is 1
# A tap on the notebook's bar hides the shared bar; the hidden state survives (status_on).
tap 930 40
wait 300
expect settings.status_on is 0
expect status.visible is 0
expect quick is 0
shot 05-notebook-bar-hidden
# And the Settings button brings the bar back after the restart, from the still-global panel.
touch down 930 40
touch move 930 140
wait 150
expect quick is 1
touch up 930 140
wait 100
tap panel quick.settings
wait 300
tap library status-bar
wait 200
tap library display.show
wait 200
expect settings.status_on is 1
tap library display.done
wait 300
expect editor.modal is none
expect status.visible is 1
expect status.reserved is 0
shot 06-notebook-status-back
quit
