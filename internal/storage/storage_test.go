package storage_test

import (
	"os"
	"testing"

	"github.com/clintjedwards/gofer/internal/storage/bolt"
)

// TODO(clintjedwards): Iterate through all implementations and make sure they return records
// in ways we expect.

func TestBolt(t *testing.T) {
	_, err := bolt.New("/tmp/test_bolt.db", 100)
	if err != nil {
		t.Fatal(err)
	}

	defer os.Remove("/tmp/test_bolt.db")
}
