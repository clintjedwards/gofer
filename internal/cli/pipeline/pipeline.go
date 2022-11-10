package pipeline

import (
	"github.com/spf13/cobra"
)

type configLanguage string

const (
	configLanguageRust   configLanguage = "RUST"
	configLanguageGolang configLanguage = "GOLANG"
)

var CmdPipeline = &cobra.Command{
	Use:   "pipeline",
	Short: "Manage pipelines",
	Long: `Manage pipelines.

A "pipeline" is a directed acyclic graph of tasks that run together. A single execution of a pipeline is called a
"run".`,
}
