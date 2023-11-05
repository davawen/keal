# Keal

A fast application launcher, that works under wayland, with the convenience and extensibility of ULauncher, but without the cruft or the bugs.  
(or at least, that's my goals ^^')

## Config

TODO: configuration

## Plugins

Plugins are placed in `~/.config/keal/plugins/<name>`.
Characteristics are described in a `config.ini` file:
```ini
[plugin]
prefix = sm ; What the user needs to type
comment = Manage current session ; (optional) Comment shown on the right
icon = user ; (optional) Plugin icon
exec = exec.sh ; Executable, from the plugin's directory
type = text ; text or json
```

Plugins communicate via `stdio`, the goal is for plugins to be as simple and universal as possible.  

### Text plugins
Text plugins are for small, dmenu like utils:
- Keal starts the plugin
- The plugin responds with a list of choices (newline separated)
- Keal sends what choice was selected
- The plugin responds with which action keal should take, and reacts.

Concretely, here is how communication looks like:
```
(start up)
<- name:firefox
<- icon:com.firefox.icon
<- name:chromium
<- comment:Google's browser
<- name:edge
<- end
-> 0
<- fork
(launches firefox)
```

Different options are indicated by a field name, a colon, and a value.
Choices are started with `name:`, and you can optionally add an icon or a comment.
When the choice list is over, send out `end`, and wait on stdin for a response.

Here are the actions keal can take:
- `fork`: Closes the window, and continue the plugin as a separate process
    Use this if you wish to launch an application from the plugin
- `wait_and_close`: Wait for the plugin to end before closing the window
- `change_input:<value>`: Change's the input field (including plugin prefix) to the string following the colon
- `change_query:<value>`: Same as `change_input`, but keeps plugin prefix

### JSON plugins (not implemented yet)
JSON plugins involve more machinery, but allow much more interactivity.  
At the start, you send out a list of events you want to be subscribed to, then an initial list of choices.  
Follows a discussion where keal sends an event, and the plugin responds with an action:

```
<- [ "query", "enter", "shift-enter" ]
<- [
    { "name": "/" }, { "name": "~" }
]
-> { "event": "query", "value": "~" }
<- {
    "action": "update-all",
    "value": [
        { "name": "~/Documents" }, { "name": "~/Pictures" }, ...
    ]
}
-> { "event": "enter", "value": 1 }
<- {
    "action": "update-all",
    "value": [
        { "name": "~/Pictures/Photos" }, ...
    ]
}
-> { "event": "shift-enter", "value": 0 }
<- { "action": "fork" }
(Launches file explorer)
```

A choice is a JSON object with a name field, and optional icon and comment fields.
JSON plugins support the same actions as text plugins, but additionaly allow changing the original choice list:
- ```json
{
    "action": "update",
    "value": [ <index>, <choice> ]
}
```
- ```json
{
    "action": "update-all",
    "value": [ <choices>... ]
}
```
