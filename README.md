# scripter
[![Builds and tests](https://github.com/gameraccoon/scripter/actions/workflows/rust.yml/badge.svg)](https://github.com/gameraccoon/scripter/actions/workflows/rust.yml)

A small and lightweight GUI tool for automation of.. well, of automation.

- Have a lot of scripts that you run daily?  
- Wish there was something nicer than `script1.sh && script2.sh` to run scripts in a sequence?  
- Getting a combinatorial explosion of the number of batch files to run all the common script combinations?  

You are in the right place.

- No more time wasted because you missed when one script finished and didn't start the next script right away.  
- No more focus lost on maintaining of the running scripts.  
- No more trying to figure out which script failed, or scrolling up to see log of a specific script in a batch.  

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
1. Either add the tool location to PATH environment variable or make an alias/script to run it from the terminal

#### Building manually

1. Clone the repository
1. Build, copy `script_config.json` from `data` folder next to the built executable
1. Add the scripts that you are planning to use and their default parameters to the config file
1. Run the executable

## Usage

1. Navigate to the working directory where you want to run the scripts from
1. Run the scripter executable from the terminal
1. Add scripts to the list of scripts to run (specify arguments if needed)
1. Start the execution

## Screenshots
![image_2023-05-01_22-16-01](https://user-images.githubusercontent.com/24990031/235530861-ef51677f-b0cc-4b48-b690-c1fcccf68bd4.png)


## License

This project is licensed under the MIT license.
