package sqlite

import (
	"os"
	"testing"

	"github.com/clintjedwards/gofer/internal/objectStore"
	"github.com/google/go-cmp/cmp"
)

func TestSqlite(t *testing.T) {
	store, err := New("/tmp/test_sqlite_objectStore.db")
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove("/tmp/test_sqlite_objectStore.db")
	defer os.Remove("/tmp/test_sqlite_objectStore.db-wal")
	defer os.Remove("/tmp/test_sqlite_objectStore.db-shm")

	err = store.PutObject("testkey1", []byte("mysupersecretkey"), false)
	if err != nil {
		t.Fatal(err)
	}

	err = store.PutObject("testkey2", []byte("myothersupersecretkey"), false)
	if err != nil {
		t.Fatal(err)
	}

	err = store.PutObject("differentkey2", []byte("mynextsupersecretkey"), false)
	if err != nil {
		t.Fatal(err)
	}

	object, err := store.GetObject("testkey1")
	if err != nil {
		t.Fatal(err)
	}

	diff := cmp.Diff(object, []byte("mysupersecretkey"))
	if diff != "" {
		t.Errorf("result is different than expected(-want +got):\n%s", diff)
	}

	keys, err := store.ListObjectKeys("testkey")
	if err != nil {
		t.Fatal(err)
	}

	if len(keys) != 2 {
		t.Fatalf("expected two keys got %d", len(keys))
	}

	_, err = store.GetObject("doesnotexist")
	if err != objectStore.ErrEntityNotFound {
		t.Fatalf("Expected error %q; found %v", objectStore.ErrEntityNotFound, err)
	}
}
