# settings: status_on=0
# settings: status_in_notebooks=0
# settings: status_size=1
# rust-only
# The status bar's hidden state is a notebook state: a start with status_on=0 (and notebooks off)
# keeps the home screen's bar, while a notebook hides it. The top strip still opens quick settings.
wait 500
expect screen is library
expect settings.status_on is 0
expect settings.status_in_notebooks is 0
expect status.visible is 1
expect status.zone is 0 0 1860 78
expect quick is 0
shot 00-home-bar-at-start
touch down 930 40
touch move 930 140
wait 150
expect quick is 1
touch up 930 140
wait 100
expect quick.rect is 0 78 1860 556
shot 01-quick-with-bar
tap panel quick.close
wait 200
expect quick is 0
# Into a notebook: the saved hidden state applies there.
tap library add
wait 100
tap library add.new-notebook
wait 100
tap 510 460
tap 1564 2254
wait 300
expect screen is editor
expect status.visible is 0
expect status.zone is 0 0 0 0
shot 02-notebook-hidden
# A tap in the hidden bar's strip brings it back, whatever hid it.
tap 930 40
wait 300
expect settings.status_on is 1
expect settings.status_in_notebooks is 1
expect status.visible is 1
expect status.zone is 0 0 1860 78
shot 03-notebook-status-back
quit
