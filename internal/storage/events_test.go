package storage

import (
	"os"
	"testing"

	"github.com/google/go-cmp/cmp"
)

func TestCRUDEvents(t *testing.T) {
	path := tempFile()
	db, err := New(path, 200)
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(path)

	event := Event{
		Type:    "Kind",
		Details: "Some detail",
		Emitted: 0,
	}

	id, err := db.InsertEvent(db, &event)
	if err != nil {
		t.Fatal(err)
	}

	if id != 1 {
		t.Errorf("ID failed to auto-increment. expected id %d; got id %d", 1, id)
	}

	event.ID = 1

	events, err := db.ListEvents(db, 0, 0, false)
	if err != nil {
		t.Fatal(err)
	}

	if len(events) != 1 {
		t.Errorf("expected 1 element in list found %d", len(events))
	}

	if diff := cmp.Diff(event, events[0]); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	event, err = db.GetEvent(db, 1)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(event, event); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}
}
