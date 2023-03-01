# CLI Package

The CLI package provides a CLI interface for Gofer used to interact and manipulate the Gofer service. It's written with
[Cobra](https://github.com/spf13/cobra). It also serves as the entrypoint for the app in general since we run the
application by simply calling it from the CLI.

The most confusing part of this package is how global flags are handled in relation to configuration.
The intention for the global flags is that they are able to overwrite some of the main configurations of the
CLI at the user's convenience.

1. First we start out by adding a new PersistentGlobalFlag to the `RootCmd` struct. This tells the CLI that it should
   support a new global flag.
2. We then define the global variable in the `config` package, this includes the variable in the structure that all
   CLI commands read from.
3. We then define it's default value(if any) in the related `Default<>Config` function.
4. This gives us the ability to call the Init<>Config function which first reads from the configuration file(if found),
   and then the environment(and overwrites in that order if duplicate keys are found).
5. Lastly we call our global flag and if the user has set it we overwrite whatever is current in the state config.

This makes it so that each command line instance can call `InitState` and have a proper hierarchy of variable overwriting.

This is in addition to the configuration hierarchy that is also created. Explained below:

In order to implement proper configuration management for both the API and the CLI we follow [12-factor](https://12factor.net/).
The gist is that environment variables are king, but there are some nuance that this package iterates on.

The final structure for the configuration hierarchy is: file -> env -> flags. Each overrides the last step if there
are any conflicting keys.

That is to say that if a user runs a command with a --token flag. That flag will replace the GOFER_TOKEN environment variable
if one was mentioned and if there wasn't then it will replace the `gofer_token` configuration file setting if that existed.
