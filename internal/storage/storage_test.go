package storage

import (
	"os"
)

func tempFile() string {
	f, err := os.CreateTemp("", "gofer-test-")
	if err != nil {
		panic(err)
	}
	if err := f.Close(); err != nil {
		panic(err)
	}
	if err := os.Remove(f.Name()); err != nil {
		panic(err)
	}
	return f.Name()
}
