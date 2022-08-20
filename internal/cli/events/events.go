package events

import (
	"github.com/spf13/cobra"
)

var CmdEvents = &cobra.Command{
	Use:   "events",
	Short: "Get details on Gofer events",
}
