package config

import (
	"github.com/spf13/cobra"
)

var CmdPipelineConfig = &cobra.Command{
	Use:   "config",
	Short: "Manage pipeline configs",
	Long:  `Manage pipeline configurations.`,
}
