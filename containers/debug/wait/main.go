package main

import (
	"log"
	"os"
	"time"
)

func main() {
	durationStr := os.Getenv("WAIT_DURATION")
	duration, err := time.ParseDuration(durationStr)
	if err != nil {
		log.Println(err)
		os.Exit(42)
	}

	log.Printf("waiting duration %q before exiting", durationStr)
	time.Sleep(duration)
}
