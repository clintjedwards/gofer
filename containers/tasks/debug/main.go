package main

import (
	"fmt"
	"os"
	"strconv"
	"strings"

	sdk "github.com/clintjedwards/gofer/sdk/go/plugins"
	"github.com/rs/zerolog/log"
)

// Whether or not to attempt to filter out any env vars with the words key or token.
const ConfigFilter = "filter"

type commonTask struct {
	filter bool
}

func newCommonTask() (*commonTask, error) {
	return &commonTask{
		filter: false,
	}, nil
}

func (c *commonTask) Run() {
	filterStr := sdk.GetConfig(ConfigFilter)
	filter, err := strconv.ParseBool(filterStr)
	if err != nil {
		log.Error().Msgf("could not convert config %q to boolean; given string %q should be either 'true' or 'false'", ConfigFilter, filterStr)
	}

	c.filter = filter

	fmt.Println("Printing out env vars")

	if c.filter {
		fmt.Println("Filtering those that contain the words key and/or token")
	}

	envs := os.Environ()
	for _, env := range envs {
		if c.filter {
			if strings.Contains(strings.ToLower(env), "key") || strings.Contains(strings.ToLower(env), "token") {
				continue
			}
		}

		fmt.Println(env)
	}
}

func installInstructions() sdk.InstallInstructions {
	instructions := sdk.NewInstructionsBuilder()
	instructions = instructions.AddMessage(":: The debug common task allows for the testing of common tasks. "+
		"It simply prints all environment variables found in the current environment.").AddMessage("").
		AddMessage("The only setting for this common task is the ability to filter environment variables that might contain "+
			"the words 'key' or 'token'.").
		AddQuery("Turn on the filter? [true/false]", ConfigFilter)

	return instructions
}

func main() {
	task, err := newCommonTask()
	if err != nil {
		panic(err)
	}

	sdk.NewCommonTaskPlugin(task, installInstructions())
}
