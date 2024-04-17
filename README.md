# Please

Have you ever been in the situation where you need to recall those couple of commands
again and end up searching for them time after time from the internet?  

**Please** is here to solve this problem and boost your workflow. 
With please you can effortlessly build simple scripts so you dont ever have to remember those commands again.
Please is almost like vim macros but for shell commands.

## Features

With please you can: 
- build scripts
- run your scripts
- edit your scripts

## How it works

Please leverages history files to build scripts. So you have to type your commands
once and then you can forget about them.

Built scripts are stored in `~/.local/state/please/scripts`.

A simple build file is used for storing the script name and data about variables if they
are used. This file is stored in `~/.local/state/please/`.

## Installation

```
cargo install --git https://github.com/ollivarila/please.git
```

**Supported shells**: Please is currenly only implemented for zsh.
You should be able to easily implement any shell by creating a history parser for it.

## Usage

The help command provides details about the usage of this program

```sh
please --help
```

### Building
Start building a script with:

```sh
please build <script name>
```

Then you simply run your commands in the terminal 
and when you are done you run:

```sh
please build
```

### Taking input

If you need to take user input during the execution of the script you
can do that with:

```sh
please ask <prompt>
```

Please will then ask for a variable name, expression and value to use for this time,
then it will execute the command with the expression and the given value.

Example:

```console
please ask "What is your name?"
Variable name?: NAME
Expression with `NAME`?: echo "Hello $NAME"
Value to use now?: foo
Hello foo
```

Then after you build, the script will have the following content:

```sh
read -p "What is your name? " NAME
echo "Hello $NAME"
```


### Running
*I recommend that you always check the script that was built before running it for the first time!*

You can run a script with:

```sh
please run <script name>
```
or

```sh
please <script name>
```

### Editing

Edit a script with:

```sh
please edit <script name>
```

This will open then script in your preferred editor based on `EDITOR` environment variable among other things.
Check out [dialoguer](https://docs.rs/dialoguer/latest/dialoguer/struct.Editor.html) for more details.

### Deleting

You can delete a script with 
```sh
please delete <script name>
```
