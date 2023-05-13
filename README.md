# scripter
![scripter_small](https://user-images.githubusercontent.com/24990031/236623413-3db76595-c6df-4a23-bc7b-afb973204be3.gif)

[![Builds and tests](https://github.com/gameraccoon/scripter/actions/workflows/rust.yml/badge.svg)](https://github.com/gameraccoon/scripter/actions/workflows/rust.yml)

A simple and lightweight GUI tool for automation of.. well, of automation.

- Have a lot of scripts that you run daily?  
- Wish there was something nicer than `script1.sh && script2.sh` to run scripts in a sequence?  
- Getting a combinatorial explosion of the number of batch files to run all the common script combinations?  

You are in the right place.

- No more time wasted because you missed when one script finished and didn't start the next script right away.  
- No more focus lost on maintaining of the running scripts.  
- No more figuring out which script failed, or scrolling up to see log of a specific script in a batch.  

Now you can schedule the exact combination of scripts to run in just a few clicks. You can go for lunch or continue working on something else, knowing that the work will be done even without your active involvement.  

## Notable features

- Queue the execution of the specific chain of scripts that you need right now, just in a few clicks
- Specify arguments, retry count, and some other parameters if needed
- Once configure what scripts you can run, and not deal with configuraion files ever again
- See the state of the execution, or open the full logs to see the details

## Getting Started

### Prerequisites

- Operating system: Windows or Linux (Mac is not tested)

### Installation

#### From Releases
1. Download a version from the releases page
1. Copy the executable to a location that the script can have write access to
1. Open `scripter_config.json` and add the scripts you are planning to use and their default parameters
1. Prepare scripter to be run from your working directory if needed:
    1. either add the tool location to PATH environment variable
    1. or make an alias/script to run it from the terminal
    1. or create a Windows shortcut to run it in the desired folder
    1. or provide `--work-path your_path` to the executable when running

#### Building manually

1. Clone the repository
1. Build, copy `script_config.json` from `data` folder next to the built executable
1. Add the scripts that you are planning to use and their default parameters to the config file

## Usage

1. Run the scripter executable the way you configured before
1. Add the scripts you want to run to the queue, and specify their arguments if needed
1. Start the execution

### Available arguments
- `--config-path <path>` - path to the json file with the configuration of scripter that should be used for this instance
- `--work_path <path>` - path to the working directory that will be used to execute the scripts
- `--logs-path <path>` - path to the folder where logs will be stored (requires write access)
- `--env <key> <value>` - specify an environment variable that will be set to every sctipt (can have multiple `--env` arguments)
- `--title <title>` - specify an additional line of title that goes under the path in Execution tab

### Available configurations

#### Global
- `always_on_top` - true of false, specifies whether the window should try to be on top of other windows
- `custom_theme` - specifies custom colors that forms a visual theme

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

#### Per script

- `name` - name of the script that will be shown in the list
- `command` - path to a script, or name of a command that is going to be executed
- `arguments` - list of arguments that is going to be passed to the scrip or the command
- `path_relative_to_scripter` - whether the path for the script should be relatie to the executable (instead of working directory from where it was called)
- `autorerun_count` - how many times the script will be retied before failing the execution
- `ignore_previous_failures` - should this script be executed even if a script before failed

### Tips and tricks

I wanted to keep the tool simple but at the same time useful for different situations. Every use case is a bit special, and here are some tricks you can do to achieve some desired behaviors (please share if you still lack some configuration options).

- You can run console commands from scripter as well, for example you can set "git" as the "command" in the configuration and be able to schedule any git command by changing the arguments before running it.
- You can make a script being run even if there was a failure before. Set "Ignore previous failures" checkbox or set the default value in the config.   
This allows to set up "notification" scripts that play a sound, show a message, or send an email to you when the list is finished, regardless of the outcome of the run.
- You can set up a script to try again if it fails. Set a positive value to "Retry count" when you add a script to a run, or set the default value in the config.  
This allows to more reliably run scripts that depend on stable internet connection. It would be a waste of time to run scripts to prepare freshly built branch in the evening, and then find in the morning that "git pull" failed because the network was unstable.
- You can specify commands relative to the scripter executable in the config, setting "path_relative_to_scripter" parameter to true.  
This allows to bundle scripter with the scripts to share with other developers, and allowing everyone who gets your tools to have the same experience regardles of their local setup.
- As arguments to scripter you can provide both the path to the configuration file and the path to folder where logs will be stored.  
This makes it possible to have multiple lists of available scripts, or keep a split between bin/etc/temp folders.
- You can specify environment variables for scripts when you run scripter  
This makes it possible to run the same scripts in different configurations (e.g. compiling in Debug/Release) and fine-tune the level of configurability. Using --title argument also allows to show the information about current context of the execution to the user of your scripts.

## Screenshots
![20230505_222428_scripter_L55OnW](https://user-images.githubusercontent.com/24990031/236622895-97782150-fa07-419e-acdc-9550d35e0407.png)
![20230506_135829_scripter_AzxYkV](https://user-images.githubusercontent.com/24990031/236622897-4a7c9a67-1976-4cfe-b147-6a93f9406d9a.png)

## License

This project is licensed under the MIT license.
