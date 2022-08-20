package pipelines

import (
	"github.com/spf13/cobra"
)

type configLanguage string

const (
	configLanguageRust   configLanguage = "RUST"
	configLanguageGolang configLanguage = "GOLANG"
)

var CmdPipelines = &cobra.Command{
	Use:   "pipelines",
	Short: "Manage pipelines",
	Long: `Manage pipelines.

A "pipeline" is a directed acyclic graph of tasks that run together. A single execution of a pipeline is called a
"run".`,
}
