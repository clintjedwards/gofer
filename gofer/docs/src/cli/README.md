# Command Line

Gofer's main way of providing interaction is through a command line application included in the Gofer binary.

This command line tool is how you upload pipelines, view runs, upload artifacts and many other common Gofer tasks.

To view the possible commands for the Gofer pipeline simply run `gofer --help`.

## Opening the web UI

`gofer web` opens Gofer's web UI in your browser, already signed in with the token your CLI uses.

```sh
gofer web                # the front page
gofer web my-pipeline    # that pipeline's latest run
gofer web my-pipeline 3  # run 3 of that pipeline
```

Your token never goes in the link. The CLI asks Gofer for a one time code that expires after 60 seconds, and the
page trades it for your token when it loads. Use `--print` to get the link instead of opening a browser, which is
handy over SSH. If the CLI doesn't have a working token, the page opens without signing in, and you can still browse
the default namespace.
