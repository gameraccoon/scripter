# scripter
![scripter_small](https://github.com/gameraccoon/scripter/assets/24990031/c2346d05-421f-4c9a-8c70-81e8c7794aef)

[![Builds and tests](https://github.com/gameraccoon/scripter/actions/workflows/rust.yml/badge.svg)](https://github.com/gameraccoon/scripter/actions/workflows/rust.yml)

A simple and lightweight GUI tool for automation of.. well, of automation.

- Have a lot of scripts that you run daily?  
- Wish there was something nicer than `script1.sh && script2.sh` to run scripts in a sequence?  
- Getting a combinatorial explosion of the number of batch files to run all the common script combinations?  

You are in the right place.

- No more time wasted because you missed when one script finished and didn't start the next script right away.  
- No more focus lost on maintaining the running scripts.  
- No more figuring out which script failed, or scrolling up to see the log of a specific script in a batch.  

Now you can schedule the exact combination of scripts to run in just a few clicks. You can go for lunch or continue working on something else, knowing that the work will be done even without your active involvement.  

## Notable features

- Queue the execution of the specific chain of scripts that you need right now, just in a few clicks
- Specify arguments, retry count, and some other parameters if needed
- Configure what scripts you can run once. No need to edit scripts/config files manually before each unique run
- See the state of the execution, or open the complete logs to see the details

## Getting Started

### Prerequisites

- Operating system: Windows, Mac or Linux

### Installation

#### From Releases
1. Download a version from the releases page and unzip it
1. Run scripter, press the "Edit" button, and add the scripts you want to run through it
1. Prepare scripter to be run from the appropriate working directory if needed in one of the ways:
    1. either add the tool location to the PATH environment variable
    1. or make an alias/script to run it from the terminal
    1. or create a Windows shortcut to run it in the desired directory
    1. or provide `--work-path your_path` to the executable when running

#### Building manually

1. Clone the repository
1. Build using `cargo build --release`
2. Copy `script_config.json` from the `data/common` directory next to the built executable
3. Run scripter, press the "Edit" button, and add the scripts you want to run through it

## Usage

1. Run the scripter executable the way you configured it before
1. Add the scripts you want to run to the queue and specify their arguments if needed
1. Start the execution

### Available arguments
- `--config-path <path>` - path to the JSON file with the configuration of scripter that should be used for this instance
- `--work_path <path>` - path to the working directory that will be used to execute the scripts
- `--logs-path <path>` - path to the directory where logs will be stored (requires write access)
- `--env <key> <value>` - specify an environment variable that will be set to every script (can have multiple `--env` arguments)
- `--title <title>` - specify an additional line of title that goes under the path in the Execution tab
- `--icons-path <path>` - path to the directory that contains the app icons (if not specified, icon paths will be relative to the scripter directory)

## Advanced usage

I wanted to keep the tool simple but at the same time useful for different situations. Every use case is a bit special, and here are some tricks you can do to achieve some desired behaviors (please share if you still lack some configuration options).

- You can run console commands from scripter as well, for example, you can set "git" as the "command" in the configuration and be able to schedule any git command by changing the arguments before running it.
- You can make a script run even if there was a failure before. Set the "Ignore previous failures" checkbox or set the default value in the config.   
This allows you to set up "notification" scripts that play a sound, show a message, or send an email to you when the list is finished, regardless of the outcome of the run.
- You can set up a script to try again if it fails. Set a positive value to "Retry count" when you add a script to a run, or set the default value in the config.  
This allows to more reliably run scripts that depend on a stable internet connection. It would be a waste of time to run scripts to prepare freshly built branches in the evening, and then find in the morning that "git pull" failed because the network was unstable.
- You can specify commands relative to the scripter executable in the config, setting the "path_relative_to_scripter" parameter to true.  
This allows bundling scripter with the scripts to share with other developers and allows everyone who gets your tools to have the same experience regardless of their local setup.
- As arguments to scripter you can provide both the path to the configuration file and the path to the directory where logs will be stored.  
This makes it possible to have multiple lists of available scripts or keep a split between bin/etc/temp directories.
- You can specify environment variables for scripts when you run scripter  
This makes it possible to run the same scripts in different configurations (e.g. compiling in Debug/Release) and fine-tune the level of configurability. Using --title argument also allows you to show the information about the current context of the execution to the user of your scripts.
- You can specify a path for a "child" config, splitting the config into two parts: parent and config, that can be edited independently.  
This allows users of your scripts to add their own scripts without affecting the config you ship to them, and without ever needing to care about how they update their script definitions


## Manual configurations

### Global
- `always_on_top` - true or false, specifies whether the window should try to be on top of other windows
- `window_status_reactions` should scripter blink the icon in the taskbar when finished
- `icon_path_relative_to_scripter` - true or false, specify whether the path for icons should be relative to scripter (true) or to the working directory (false). this option is ignored when `--icons-path` argument is provided.
- `keep_window_size` - set to true if you don't want the app to change the window size (e.g. if it doesn't work well with your window manager)
- `custom_theme` - specifies custom colors that form a visual theme
- `child_config_path` - path to the "child" config that can be used for having local changes that don't affect the main config (e.g if the main config is shared between developers)

Example of a dark theme:
```json
"custom_theme": {
	"background": [0.25, 0.26, 0.29],
	"text": [0.0, 0.0, 0.0],
	"primary": [0.44, 0.53, 0.855],
	"success": [0.31, 0.50, 0.17],
	"danger": [1.0, 0.0, 0.0]
}
```

### Per script

- `uid` - a unique script identifier (UUID v4 is used by default)
- `name` - the name of the script that will be shown in the list
- `icon` - optional path to the icon that will be shown next to the script name
- `command` - path to a script, or name of a command that is going to be executed
- `arguments` - list of arguments that are going to be passed to the script or the command
- `path_relative_to_scripter` - whether the path for the script/command should be relative to the scripter executable directory (instead of the working directory from where it was called)
- `autorerun_count` - how many times the script will be retried before failing the execution
- `ignore_previous_failures` - should this script be executed even if a script before failed


## Screenshots
![image](https://github.com/gameraccoon/scripter/assets/24990031/ef21a887-e902-406f-af00-38411c383e27)
![image](https://github.com/gameraccoon/scripter/assets/24990031/442c17bc-5f72-4fe6-ad63-098bd60fb882)


## License

This project is licensed under the MIT license.
