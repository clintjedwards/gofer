package pipeline

import (
	"github.com/spf13/cobra"
)

var CmdPipelineSecret = &cobra.Command{
	Use:   "secret",
	Short: "Store pipeline specific secrets",
	Long: `Store pipeline specific secrets.

Gofer allows you to store pipeline secrets. These secrets are then used to populate the pipeline
configuration file.
`,
}

func init() {
	CmdPipeline.AddCommand(CmdPipelineSecret)
}
