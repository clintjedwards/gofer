package secrets

import (
	"github.com/spf13/cobra"
)

var CmdSecrets = &cobra.Command{
	Use:   "secrets",
	Short: "Get details about Gofer secrets",
	Long:  `Get details about Gofer secrets.`,
}
