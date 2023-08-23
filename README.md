# scripter
![scripter_animation](https://github.com/gameraccoon/scripter/assets/24990031/39a17a9e-0835-49a5-910e-62785a48ec98)

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
- Configure what scripts you can run once. No need to edit scripts or configs manually
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

### Available arguments (advanced)
- `--config-path <path>` - path to the JSON file with the configuration of scripter that should be used for this instance
- `--work_path <path>` - path to the working directory that will be used to execute the scripts
- `--logs-path <path>` - path to the directory where logs will be stored (requires write access)
- `--env <key> <value>` - specify an environment variable that will be set to every script (can have multiple `--env` arguments)
- `--title <title>` - specify an additional line of title that goes under the path in the Execution tab

## Advanced usage cases

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

## Screenshots
![20230823_215456_scripter_CEVt1P](https://github.com/gameraccoon/scripter/assets/24990031/2d5fc8e0-f4ae-4919-b108-bbd475f03a70)
![20230823_215906_scripter_nvHOt7](https://github.com/gameraccoon/scripter/assets/24990031/abcff320-c2c1-48d8-a4e8-86d3577164bc)

## License

This project is licensed under the MIT license.
