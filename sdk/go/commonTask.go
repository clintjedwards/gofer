package sdk

import (
	"fmt"
	"os"

	"github.com/rs/zerolog/log"
)

type CommonTaskPluginInterface interface {
	Run()
}

func NewCommonTaskPlugin(service CommonTaskPluginInterface, installInstructions InstallInstructions) {
	if len(os.Args) != 2 {
		log.Fatal().Msg("Usage: ./commontask <task|installer>")
	}

	switch os.Args[1] {
	case "task":
		service.Run()
	case "installer":
		instructions, err := installInstructions.JSON()
		if err != nil {
			log.Fatal().Msg("could not parse instructions to json")
		}
		fmt.Println(instructions)
	default:
		log.Fatal().Msg("Usage: ./commontask <task|installer>")
	}
}
