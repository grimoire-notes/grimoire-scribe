# settings: status_mode=1
# rust-only
# root: z_widget_host
# Grimoire started again on z_widget_host's data: the frame it left open, of the package whose id is 128 bytes
# long, is open again where it was dragged to, read back from widgets.ini by the rule the manifest check has for
# an id (issue #208).
wait 3600
expect screen is library
tap 326 608
wait 500
expect screen is editor
expect editor.widgets is 1 test.host.long.0123456789-abcdefghij.0123456789-abcdefghij.0123456789-abcdefghij.0123456789-abcdefghij.0123456789-abcdefghij.01z
expect editor.widget.0.state is normal
expect editor.widget.0.rect is 776 668 400 300
expect editor.widget.0.error is
expect editor.widget.0.text is hello
shot 01-long-again
tap panel widget.0.close
wait 300
expect editor.widgets is 0
quit
