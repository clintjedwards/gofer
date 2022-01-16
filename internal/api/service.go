package api

import (
	"strings"
)

var appVersion = "0.0.dev_000000_33333"

func parseVersion(versionString string) (version, commit, buildTime string) {
	splitVersion := strings.Split(versionString, "_")

	version = splitVersion[0]
	commit = splitVersion[1]
	buildTime = splitVersion[2]

	return
}
