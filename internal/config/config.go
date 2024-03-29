// Config controls the overall configuration of the application.
//
// It is generated by first attempting to read a configuration file and then overwriting those values
// with anything found in environment variables. Environment variables always come last and have the highest priority.
// As per (https://12factor.net/config).
//
// All environment variables are prefixed with "GOFER". Ex: GOFER_DEBUG=true
//
// Most envvar configuration abides by common envvar string=string formatting: Ex. `export GOFER_DEBUG=true`.
// Complex envvars might take a json string as value.
//
//	Example: export GOFER_EXTENSIONS="{"name":"test"},{"name":"test2"}"
//
// You can print out a current description of current environment variable configuration by using the cli command:
//
//	`gofer service printenv`
package config

import (
	"errors"
	"log"
	"os"
	"reflect"
	"strings"
	"time"

	"github.com/fatih/structs"
)

func mustParseDuration(duration string) time.Duration {
	parsedDuration, err := time.ParseDuration(duration)
	if err != nil {
		log.Fatalf("could not parse duration %q; %v", duration, err)
	}

	return parsedDuration
}

// searchFilePaths will search each path given in order for a file
//
//	and return the first path that exists.
func searchFilePaths(paths ...string) string {
	for _, path := range paths {
		if path == "" {
			continue
		}

		stat, err := os.Stat(path)

		if errors.Is(err, os.ErrNotExist) {
			continue
		}

		if stat.IsDir() {
			continue
		}
		return path

	}

	return ""
}

func getEnvVarsFromStruct(prefix string, fields []*structs.Field) []string {
	output := []string{}

	for _, field := range fields {
		tag := field.Tag("koanf")
		if field.Kind() == reflect.Pointer {
			output = append(output, getEnvVarsFromStruct(strings.ToUpper(prefix+tag+"__"), field.Fields())...)
			continue
		}

		output = append(output, strings.ToUpper(prefix+tag))
	}

	return output
}
