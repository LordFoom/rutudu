# rutudu
The simple, tui rust to do list. You can say it roo-too-doo or rah-tah-dah, or even rah-too-doo!

Work in progress but you can:

* Make simple list, with entry text
* Sub lists.

Wanted a TUI based todo list with hierarchies that doesn't just cat to the console,
but allows navigation,collapsing, etc. Couldn't find one so here we are.

You cannot edit and you cannot delete. 

I may never support this, to give it more of a typewriter resistance to change, 
maybe it'll help me think before I speak?

Or maybe I will end up supporting it.

You can cross-out and and uncross-out.

Will auto open a new list with today's date, creating if necessary, if started with no arguments. Otherwise will
open/create passed in list name.

### You can add items!

* a to add an item
* Ctrl+a to add a sub item
* Alt+a to add new parent
* __It is CTRL+N  to e(N)ter on the add item screen__
  * That's because enter is for newlines when creating items.
* Seriously, if somebody can teach me how to capture CTRL+enter, I'd be so grateful
  * Can't seem to modify enter on the terminal?

### Can do things with the list
* x to (un)cross out item
* s to save as sqlite file 
* o to load (up/down to select, press right/enter/ctrl+n to open)



I like having lots of short lists and they're stored for posterity in sqlite. Maybe in future, some kind of tool
to aggregate lists? W

Will integrate with my clockrust, my nascent timetracking project. In time. In time.

### ROADMAP

* Move items up/down
* edit? delete? AND GIVE IN TO THE EPHEMERAL??

<img src="./example_pic.png" title="Looks like this" >
<img src="./grey_crossed_out.png" title="Now with soothing crossed out items" >
<img src="./open_file.png" title="Opening files" >
<img src="./rutud_1.gif" title="The cursor works...in the forward direction" width="1046" height="555">
<img src="./hierarchies.png" title="sub-lists"/>
