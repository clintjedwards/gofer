package secret

import (
	"github.com/spf13/cobra"
)

var CmdGlobalSecret = &cobra.Command{
	Use:   "global",
	Short: "Store global specific secrets",
	Long: `Store global specific secrets.

Gofer allows you to store global secrets. These secrets are then used to populate all the places where
Gofer needs to use shared secrets. Only accessible to admins.`,
}

func init() {
	CmdSecret.AddCommand(CmdGlobalSecret)
}
