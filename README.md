# scripter
![scripter_animation](https://github.com/gameraccoon/scripter/assets/24990031/39a17a9e-0835-49a5-910e-62785a48ec98)

[![Builds and tests](https://github.com/gameraccoon/scripter/actions/workflows/rust.yml/badge.svg)](https://github.com/gameraccoon/scripter/actions/workflows/rust.yml)

A simple and lightweight GUI tool for automation of.. well, of automation.

- Have a lot of scripts that you run daily?  
- Wish there was something nicer than `script1.sh && script2.sh` to run scripts in a sequence?  
- Getting a combinatorial explosion of the number of batch files to run all the common script combinations?  
- Wish you could share your scripts with non-programmers?  

You are in the right place.

- No more time wasted because you context-switched mid your workflow and forgot to run the next batch of scripts.
- No more focus lost on maintaining the running scripts.
- No more figuring out which script failed, or scrolling up in the terminal to see the log of a specific script in a batch.

Now you can schedule the exact combination of scripts to run in just a few clicks. You can go for lunch or continue working on something else, knowing that the work will be done even without your active involvement.  

scripter is not a tool that would force you to build your workflow around it, it is just a complimentary tool in your toolbox. Run your scripts as usual, and run them with scripter when you want to run many scripts sequentially. See how it works for you, and check out how you can configure scripter to satisfy your needs.

## Notable features

- Queue the execution of the specific chain of scripts that you need right now, just in a few clicks
- Specify arguments, retry count, and some other parameters if needed
- Configure once what scripts you can run. No need to edit scripts or configs manually
- See the state of the execution, or open the complete logs to see the details
- Save often-used script combinations into presets, run a preset in just one or two clicks
- Set up quick buttons for scripts like "open project in IDE" that are always visible

## Getting Started

### Prerequisites

- Operating system: Windows, Linux or Mac

### Installation

#### From Releases
1. Download a version from the releases page and unzip it
1. Run scripter, press the "Edit" button, and add the scripts you want to run through it
1. Prepare scripter to be run from the appropriate working directory if needed in the way that suits your workflow:
    1. either add the tool location to the PATH environment variable
    1. or make an alias/script to run it from the terminal
    1. or create a Windows shortcut to run it in the desired directory
    1. or provide `--work-path your_path` to the executable when running

#### Building manually
1. Clone the repository
1. Build using `cargo build --release`
1. Run scripter, press the "Edit" button, and add the scripts you want to run through it

## Usage

1. Run the scripter executable the way you configured it before
1. Add the scripts you want to run to the queue and specify their arguments if needed
1. Start the execution

## Advanced usage

### Command-line arguments
- `--config-path <path>` - path to the JSON file with the configuration of scripter that should be used for this instance
- `--work_path <path>` - path to the working directory that will be used to execute the scripts
- `--logs-path <path>` - path to the directory where logs will be stored (requires write access)
- `--env <key> <value>` - specify an environment variable that will be set to every script (can have multiple `--env` arguments)
- `--title <title>` - specify an additional line of title that goes under the path in the Execution tab

### Advanced usage cases

I wanted to keep the tool simple but at the same time useful for different situations. Every use case is a bit special, and here are some tricks you can do to achieve some desired behaviors (please share if you still lack some configuration options).

- You can run console commands from scripter as well, for example, you can set "git" as the "command" and be able to schedule any git command by changing the arguments before running it.
- You can make a script run even when some other script fails. Set the "Ignore previous failures" checkbox when configuring the script or before running it.   
This allows you to set up "notification" scripts that play a sound, show a message, or send a push notification to your phone when the list is finished, regardless of the outcome of the run.
- You can set up a script to try again if it fails. Set a positive value to "Retry count" when configuring the script or before running it.  
This allows to more reliably run scripts that depend on a stable internet connection. It would be a waste of time to run scripts to prepare freshly built branches in the evening, and then find in the morning that "git pull" failed because the network was unstable.
- You can specify commands relative to the scripter executable in the config, setting the "path_relative_to_scripter" parameter to true.  
This allows bundling scripter with the scripts to share with other developers and allows everyone who gets your tools to have the same experience regardless of their local setup.
- As arguments to scripter you can provide both the path to the configuration file and the path to the directory where the logs are going to be stored.  
This makes it possible to have multiple lists of available scripts or keep a split between bin/etc/temp directories.
- You can specify environment variables for scripts when you start scripter by providing `--env` arguments  
This makes it possible to run the same scripts in different configurations (e.g. compiling in Debug/Release) and fine-tune the level of configurability. Using `--title` argument also allows you to show the information about the current context of the execution to the user of your scripts.
- You can specify the path for a "local" config, splitting the config into two parts: "shared" and "local", that can be edited independently.  
This allows users of your scripts to add their own scripts without affecting the config you ship to them, and without ever needing to care about how they update their script definitions.
- You can add `.scripter_config.json` to your project folder, then when scripter is being run from that directory it will use this file as the current config.  
This allows to use scripter with differently configured projects without the need to provide the path to the correct config manually

## Screenshots
![20230823_215456_scripter_CEVt1P](https://github.com/gameraccoon/scripter/assets/24990031/2d5fc8e0-f4ae-4919-b108-bbd475f03a70)
![Screenshot from 2023-08-24 21-54-48](https://github.com/gameraccoon/scripter/assets/24990031/80478ade-fa2e-483b-90d3-ea6340222e18)

## License

This project is licensed under the MIT license.
