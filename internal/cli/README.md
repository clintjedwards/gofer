# CLI Package

The CLI package provides a CLI interface for Gofer used to interact and manipulate the Gofer service. It's written with
Cobra.

The most confusing part of this package is how global flags are handled. The intention for global flags is that are controllable by config, envvar, and user flags. The intended user experience here is for the user the be able to specify flags and envvars on
the fly when working with the CLI, but if they have a setting they'd like to keep for every CLI action
then they have the ability to specify that setting permanently in a configuration file. This model adheres to the
[12-factor](https://12factor.net/config) philosophy with a slight tweak for local usage vs app usage.

To do this:

1. First we start out by adding a new PersistentGlobalFlag to the `RootCmd` struct. This tells the CLI that it should
   support a new global flag.
2. We then define the global variable in the `config` package, this includes the variable in the structure that all
   CLI commands read from.
3. We then define it's default value(if any) in the related `Default*Config` function.
4. This gives us the ability to call the Init\*Config function which first reads from the configuration file(if found),
   and then the environment(and overwrites in that order if duplicate keys are found).
5. Lastly we call our global flag and if the user has set it we overwrite whatever is current in the state config.

This makes it so that each command line instance can call `InitState` and have a proper hierarchy of variable overwriting.

The hierarchy is: config -> envvar -> flag. With each overwriting the previous one if they keys conflict. That is to say,
if a user defines an option in their configuration file and then uses a flag that controls that option the flag's value
will be respected and the config's value will be ignored.
