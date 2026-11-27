# settings: settings_version=3
# settings: whatsnew_seen=1.2.0
# What's new opens by itself at the first start after an update (a settings file a Grimoire saved,
# its notes seen another version's): once, this version's then recorded (z_about_2_after). It opens
# full screen with this version selected in the sidebar; its X closes it.
# rust-only
wait 300
expect screen is library
expect library.sheet is about
expect library.about.view is browser
expect library.about.full is 1
expect library.about.entries is 7
expect library.about.rows is 7
expect library.about.note is 2.3.0-beta.1
expect library.about.marked is 1
expect settings.whatsnew_seen not 1.2.0
expect settings.whatsnew_seen not
shot 01-after-update
tap library about.close
wait 100
expect library.sheet is none
quit
