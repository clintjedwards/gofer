package run

import (
	"github.com/spf13/cobra"
)

var CmdRunStore = &cobra.Command{
	Use:   "store",
	Short: "Store run specific values",
	Long: `Store run specific values.

Gofer has two ways to temporarily store objects that might be useful.

This command allows users to store objects at the "run" level in a key-object fashion. Run level objects are
great for storing things that need to be cached only for the communication between tasks.

Run objects are kept individual to each run and removed after a certain run limit. This means that after a certain
amount of runs for a particular pipeline a run's objects will be discarded. The limit of amount of objects you can
store per run is of a much higher limit.
`,
}

func init() {
	CmdRun.AddCommand(CmdRunStore)
}
