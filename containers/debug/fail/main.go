package main

import (
	"os"
	"time"
)

func main() {
	time.Sleep(time.Second * 4)
	os.Exit(1337)
}
