# Countdown

## Description

Countdown is a command line program that lets you know how many days are
remaining until any number of events that you've configured. Use it in your
shell's $PS2 to always have the soonest event displayed, or just use it on the
fly whenever you need some encouragement for the week.

![demo](https://user-images.githubusercontent.com/5622404/118373813-932a0780-b56d-11eb-9388-d58adc65b8a6.gif)


## Usage

```text
USAGE:
    countdown [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -n, --n <n>            Max number of events to display.
    -o, --order <order>    Specify the ordering of the events returned [possible values: shuffle, time-asc, time-desc]
```

## Setup & Installation
1. Install with `cargo install --git https://github.com/leethomas/countdown --branch main`

2. Before running, create a `.countdown.toml` file in your home directory containing a list of events you'd like to track. Here's an example with dates far into the future (as of 2021 😄):

```toml
[[events]]
name = "summer break"
time = 1892160000 # these are unix timestamps in seconds

[[events]]
name = "wwdc"
time = 1892250000

[[events]]
name = "memorial day weekend"
time = 1892170000
```

3. Now run `countdown` in your shell and you're all set 🎉
