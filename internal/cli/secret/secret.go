package secret

import (
	"github.com/spf13/cobra"
)

var CmdSecret = &cobra.Command{
	Use:   "secret",
	Short: "Get details about Gofer secrets",
	Long:  `Get details about Gofer secrets.`,
}
