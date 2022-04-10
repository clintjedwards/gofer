package sdk

import (
	"fmt"
	"os"
)

func ExampleGetConfig() {
	_ = os.Setenv("GOFER_TRIGGER_KIND", "interval")
	_ = os.Setenv("GOFER_TRIGGER_INTERVAL_MIN_DURATION", "1h")

	// Config wanted is "MIN_DURATION"
	minDuration := GetConfig("min_duration")

	// The result will be whatever the value is set to via the environment.
	// The function automatically just subtitutes in the key's expected format.
	fmt.Println(minDuration)
	// Output: 1h
}
