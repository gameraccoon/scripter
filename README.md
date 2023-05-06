# scripter
![scripter_small](https://user-images.githubusercontent.com/24990031/236623413-3db76595-c6df-4a23-bc7b-afb973204be3.gif)

[![Builds and tests](https://github.com/gameraccoon/scripter/actions/workflows/rust.yml/badge.svg)](https://github.com/gameraccoon/scripter/actions/workflows/rust.yml)

A small and lightweight GUI tool for automation of.. well, of automation.

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
- Can specify arguments, retry count, and some other parameters
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
1. Either add the tool location to PATH environment variable, make an alias/script to run it from the terminal, or create a windows shortcut to run it in the desired folder

#### Building manually

1. Clone the repository
1. Build, copy `script_config.json` from `data` folder next to the built executable
1. Add the scripts that you are planning to use and their default parameters to the config file
1. Run the executable

## Usage

1. Navigate to the working directory where you want to run the scripts from
1. Run the scripter executable from the terminal
1. AAdd the scripts you want to run to the list, and specify their arguments if needed
1. Start the execution

### Tips and tricks

I wanted to keep the tool simple but at the same time useful for different situations. Every use case of the tool is special, and here are some tricks you can do to achieve some desired behaviors (please share if you still lack some configuration options).

- You can run normal console commands from scripter as well, for example you can set "git" as the "command" in the configuration and be able to schedule any git command by changing the arguments before running it.
- You can make a script being run even if there was a failure before. Set "Ignore previous failures" checkbox or set it to true as default in the configuration.  
This allows to set up "notification" scripts that play a sound, show a message, or send an email to you when the list is finished regardless of the outcome of the run.
- You can make a script try again if it fails. Set a positive value to "Retry count" when you add a script to a run, or set the default value in the config.  
This allows to more reliably run scripts that depend on stable internet connection. It would be a waste of time to run scripts to prepare freshly built branch in the evening, and then find in the morning that "git pull" failed because the network was unstable.
- You can specify commands relative to the scripter executable in the config, setting "path_relative_to_scripter" parameter to true.  
This allows to bundle scripter with the scripts to share with other developers, and allowing everyone who gets your tools to have the same experience regardles of their local setup.

## Screenshots
![20230505_222428_scripter_L55OnW](https://user-images.githubusercontent.com/24990031/236622895-97782150-fa07-419e-acdc-9550d35e0407.png)
![20230506_135829_scripter_AzxYkV](https://user-images.githubusercontent.com/24990031/236622897-4a7c9a67-1976-4cfe-b147-6a93f9406d9a.png)

## License

This project is licensed under the MIT license.
