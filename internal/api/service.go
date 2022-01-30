package api

import (
	"strings"
)

var appVersion = "0.0.dev_000000"

func parseVersion(versionString string) (version, commit string) {
	version, commit, err := strings.Cut(versionString, "_")
	if !err {
		return "", ""
	}

	return
}
