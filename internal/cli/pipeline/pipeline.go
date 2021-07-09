package pipeline

import (
	"github.com/spf13/cobra"
)

var CmdPipeline = &cobra.Command{
	Use:   "pipeline",
	Short: "Manage pipelines",
	Long: `Manage pipelines.

A "pipeline" is a directed acyclic graph of tasks that run together. A single execution of a pipeline is called a
"run".`,
}
