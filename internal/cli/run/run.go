package run

import (
	"github.com/spf13/cobra"
)

var CmdRun = &cobra.Command{
	Use:   "run",
	Short: "Manage runs",
	Long: `Manage runs.

A "run" is a single instance of a pipeline's execution. It consists of a collection of tasks that can be
all run in parallel or depend on the execution of others.`,
}
