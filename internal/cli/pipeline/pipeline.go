package pipeline

import (
	"github.com/clintjedwards/gofer/internal/cli/pipeline/config"
	"github.com/clintjedwards/gofer/internal/cli/pipeline/extension"
	"github.com/spf13/cobra"
)

var CmdPipeline = &cobra.Command{
	Use:   "pipeline",
	Short: "Manage pipelines",
	Long: `Manage pipelines.

A "pipeline" is a directed acyclic graph of tasks that run together. A single execution of a pipeline is called a
"run".`,
}

func init() {
	CmdPipeline.AddCommand(extension.CmdPipelineExtension)
	CmdPipeline.AddCommand(config.CmdPipelineConfig)
}
