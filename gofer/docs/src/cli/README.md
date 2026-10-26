# Command Line

Gofer's main way of providing interaction is through a command line application included in the Gofer binary.

This command line tool is how you upload pipelines, view runs, upload artifacts and many other common Gofer tasks.

To view the possible commands for the Gofer pipeline simply run `gofer --help`.

## Reading task logs

`gofer task logs <pipeline_id> <run_id> <task_id>` prints a task's output. If the task is still running it keeps
following the log until the task finishes, so you can watch it as it goes.

For long logs you usually only want one end of it:

```sh
gofer task logs my-pipeline 3 build --tail 50     # the last 50 lines, then keep following if it's still running
gofer task logs my-pipeline 3 build | head -n 50  # the first 50 lines
```

`--tail` is handled by the server, which skips straight to the end of the log instead of sending you the whole thing.
That makes it much faster than piping into `tail` and it works on tasks that are still running, where `tail` would sit
waiting for a log that hasn't ended yet. The start of the log needs no help from the server, so `head` works fine.

The output is plain lines, so the usual tools work on it:

```sh
gofer task logs my-pipeline 3 build | grep -i error
```

### Searching with less

For anything more than a quick look, pipe the logs into `less`:

```sh
gofer task logs my-pipeline 3 build | less -R
```

`-R` keeps any colors the task printed. Once it's open:

| Key           | What it does                                                               |
| ------------- | -------------------------------------------------------------------------- |
| `/error`      | Search forward for `error`. `n` and `N` jump to the next and previous hit. |
| `?error`      | Search backwards instead.                                                  |
| `&error`      | Only show lines containing `error`. `&` on its own shows everything again. |
| `&!healthz`   | Hide lines containing `healthz`, handy for cutting out noisy lines.        |
| `g` and `G`   | Jump to the start or end of the log.                                       |
| `F`           | Follow new lines as they come in, like `tail -f`.                          |
| `-S`          | Toggle wrapping long lines, so wide output can scroll sideways instead.    |
| `q`           | Quit.                                                                      |

Patterns are regular expressions, and searches are case sensitive unless you start `less` with `-i`.

If the task is still running, be careful with `F`. Pressing `Ctrl-C` to stop following also stops `gofer`, so the log
ends wherever it was at that moment. You can keep scrolling what you have, and run the command again to pick up the
rest.

## Opening the web UI

`gofer web` opens Gofer's web UI in your browser, already signed in with the token your CLI uses.

```sh
gofer web                # the front page
gofer web my-pipeline    # that pipeline's page: recent runs, tasks, and subscriptions
gofer web my-pipeline 3  # run 3 of that pipeline
```

Your token never goes in the link. The CLI asks Gofer for a one time code that expires after 60 seconds, and the
page trades it for your token when it loads. Use `--print` to get the link instead of opening a browser, which is
handy over SSH. If the CLI doesn't have a working token, the page opens without signing in, and you can still browse
the default namespace.
