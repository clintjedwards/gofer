package config

import "fmt"

func possibleConfigPaths(homeDir, flagPath string) []string {
	return []string{
		flagPath,
		fmt.Sprintf("%s/%s", homeDir, ".gofer_dev.hcl"),
		fmt.Sprintf("%s/%s/%s", homeDir, ".config", "gofer_dev.hcl"),
	}
}
