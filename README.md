# rutudu
The simple, tui rust to do list. You can say it roo-too-doo or rah-tah-dah, or even rah-too-doo!

Work in progress but you can:

* Make simple list, with entry text
* Sub lists.

Wanted a TUI based todo list with hierarchies that doesn't just cat to the console,
but allows navigation,collapsing, etc. Couldn't find one so here we are.

You cannot edit and you cannot delete. I may never support this, just to give it that typewriter feel.

You can cross-out and and uncross-out.

Will auto open a new list with today's date, creating if necessary, if started with no arguments. Otherwise will
open/create passed in list name.

* a to add an item
* Ctrl+a to add a sub item
* Alt+a to add new parent
* x to cross out
* s to save as sqlite file 
* o to load (up/down to select, press right/enter to open)

I like having lots of short lists and they're stored for posterity in sqlite.

Will integrate with my clockrust, my new timetracking project.

<img src="./example_pic.png" alt="Looks like this" >
<img src="./open_file.png" alt="Opening files" >
<img src="./rutud_1.gif" alt="The cursor works...in the forward direction" width="1046" height="555">
<img src="./hierarchies.png" title=""sublists"/>
