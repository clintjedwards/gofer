package config

import "fmt"

func possibleConfigPaths(homeDir, flagPath string) []string {
	return []string{
		flagPath,
		fmt.Sprintf("%s/%s", homeDir, ".gofer.hcl"),
		fmt.Sprintf("%s/%s/%s", homeDir, ".config", "gofer.hcl"),
	}
}
