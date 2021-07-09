package token

import (
	"github.com/spf13/cobra"
)

var CmdToken = &cobra.Command{
	Use:   "token",
	Short: "Manage api tokens",
}
