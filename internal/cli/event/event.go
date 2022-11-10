package event

import (
	"github.com/spf13/cobra"
)

var CmdEvent = &cobra.Command{
	Use:   "event",
	Short: "Get details on Gofer events",
}
