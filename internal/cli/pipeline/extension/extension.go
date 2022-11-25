package extension

import (
	"github.com/spf13/cobra"
)

var CmdPipelineExtension = &cobra.Command{
	Use:   "extension",
	Short: "Manage pipeline extensions",
	Long:  `Manage pipeline extension subscriptions.`,
}
