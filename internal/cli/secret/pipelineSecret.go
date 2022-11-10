package secret

import (
	"github.com/spf13/cobra"
)

var CmdPipelineSecret = &cobra.Command{
	Use:   "pipeline",
	Short: "Store pipeline specific secrets",
	Long: `Store pipeline specific secrets.

Gofer allows you to store pipeline secrets. These secrets are then used to populate the pipeline
configuration file.`,
}

func init() {
	CmdSecret.AddCommand(CmdPipelineSecret)
}
