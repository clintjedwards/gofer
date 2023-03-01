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
	"time"
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

		if stat, err := os.Stat(path); errors.Is(err, os.ErrNotExist) {
			continue
		} else {
			if stat.IsDir() {
				continue
			}
			return path
		}
	}

	return ""
}
