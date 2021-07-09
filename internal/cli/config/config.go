package config

import (
	"github.com/spf13/cobra"
)

var CmdConfig = &cobra.Command{
	Use:   "config",
	Short: "Manage pipeline configuration files",
}
