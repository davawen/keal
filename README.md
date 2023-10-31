# Keal

A fast application launcher, that works under wayland, with the convenience and extensibility of ULauncher, but without the cruft or the bugs.  
(or at least, that's my goals ^^')

## Config

TODO: write readme

## Plugins

Plugins communicate via `stdio`, the goal is for plugins to be as simple and universal as possible.  
You can configure plugins to either use text or json communication.

Text plugins are for small, dmenu like utils:
- Keal starts the plugin
- The plugin responds with a list of choices (newline separated)
- Keal sends what choice was selected or wether the user quit
- The plugin takes action

Concretely, here is how communication looks like:
```
(start up)
<- F:firefox
<- I:com.firefox.icon
<- F:chromium
<- C:Google's browser
<- F:edge
<- E:
-> firefox
(launches firefox)
```

Different options are indicated by a letter followed by a colon.
Choices are started by a capital F.
You can optionally add an icon and a comment with I and C.  
When the choice list is over, it sends out `E:`, and waits on stdin.
