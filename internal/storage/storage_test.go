package storage_test

import (
	"io/ioutil"
	"os"
	"testing"

	"github.com/clintjedwards/gofer/internal/storage/bolt"
)

// TODO(clintjedwards): Iterate through all implementations and make sure they return records
// in ways we expect.

func tempfile() string {
	f, err := ioutil.TempFile("", "bolt-")
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

func TestBolt(t *testing.T) {
	_, err := bolt.New(tempfile(), 100)
	if err != nil {
		t.Fatal(err)
	}
}
