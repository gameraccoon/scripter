# scripter
[![Builds and tests](https://github.com/gameraccoon/scripter/actions/workflows/rust.yml/badge.svg)](https://github.com/gameraccoon/scripter/actions/workflows/rust.yml)

A small and lightweight GUI tool for automation of.. well, automation.

Have a lot of scripts that you run daily?  
Wish there was something nicer to run sequences of scripts than doing `script1.sh && script2.sh`?  
Getting a combinatorial explosion of the number of batch files to run all the common script combinations?  

You are in the right place.

No more time wasted because you missed when one script finished and didn't start the next script.  
No more focus lost on maintaining of the running scripts.  
No more trying to figure out which script failed, or scrolling up to see log of a specific script in a batch.  

Now you can schedule the exact combination of scripts to run in just a few clicks. You can go for lunch or continue working on something else, knowing that the work will be done. 

## Notable features

- Allows to schedule a queue of scripts
- Allows to set command arguments for each script
- Tracks execution of the scripts and shows recent logs
- Allows to see the full logs of a specific script

## Getting Started

### Prerequisites

- Operating system: Windows or Linux

### Installation

1. Download a version from the releases page or build it with Rust
1. Copy the executable to a location that the script can have write access to
1. Add all the scripts you need to run to the "scripts" folder in that location
1. Either add the script location to PATH or make an alias/script to run it from the terminal

## Usage

1. Navigate to the working directory where you want to run the scripts from
1. Run the scripter executable from the terminal
1. Add scripts to the list of scripts to run (specify arguments if needed)
1. Start the execution

## Screenshots
![image_2023-05-01_22-16-01](https://user-images.githubusercontent.com/24990031/235530861-ef51677f-b0cc-4b48-b690-c1fcccf68bd4.png)


## License

This project is licensed under the MIT license.
