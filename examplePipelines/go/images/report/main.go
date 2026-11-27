package main

import (
	"fmt"
	"runtime"
)

func main() {
	fmt.Printf("report: compiled from Go, running on %s\n", runtime.GOARCH)
}
