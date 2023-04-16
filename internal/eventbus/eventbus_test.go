package eventbus

import (
	"os"
	"testing"
	"time"

	"github.com/clintjedwards/gofer/events"
	"github.com/clintjedwards/gofer/internal/storage"
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

func TestPublish(t *testing.T) {
	path := tempFile()
	db, err := storage.New(path, 200)
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(path)

	eb, err := New(db, time.Second*5, time.Minute*5)
	if err != nil {
		t.Fatal(err)
	}

	id := eb.Publish(events.EventNamespaceCreated{
		NamespaceID: "test_namespace",
	})

	storedEvent, err := eb.Get(id)
	if err != nil {
		t.Fatal(err)
	}

	if storedEvent.ID != id {
		t.Errorf("published event id and new event id do no match; published %d; new %d",
			storedEvent.ID, id)
	}
}

func TestSubscribe(t *testing.T) {
	path := tempFile()
	db, err := storage.New(path, 200)
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(path)

	eb, err := New(db, time.Minute*5, time.Minute*5)
	if err != nil {
		t.Fatal(err)
	}

	sub, err := eb.Subscribe(events.EventTypeNamespaceCreated)
	if err != nil {
		t.Fatal(err)
	}

	eb.Publish(events.EventNamespaceCreated{
		NamespaceID: "test_namespace_1",
	})
	eb.Publish(events.EventNamespaceCreated{
		NamespaceID: "test_namespace_2",
	})
	thirdEventID := eb.Publish(events.EventNamespaceCreated{
		NamespaceID: "test_namespace_3",
	})

	<-sub.Events
	<-sub.Events
	three := <-sub.Events
	if three.ID != thirdEventID {
		t.Errorf("published event id and new event id do no match; published %d; new %d",
			three.ID, thirdEventID)
	}
}

func TestUnsubscribe(t *testing.T) {
	path := tempFile()
	db, err := storage.New(path, 200)
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(path)

	eb, err := New(db, time.Minute*5, time.Minute*5)
	if err != nil {
		t.Fatal(err)
	}

	sub, err := eb.Subscribe(events.EventTypeNamespaceCreated)
	if err != nil {
		t.Fatal(err)
	}

	eb.Publish(events.EventNamespaceCreated{
		NamespaceID: "test_namespace_1",
	})

	eb.Unsubscribe(sub)

	if len(eb.subscribers[events.EventTypeNamespaceCreated]) != 0 {
		t.Errorf("Unsubscribe not successful: %+v", eb.subscribers[events.EventTypeNamespaceCreated])
	}
}

func TestGetAll(t *testing.T) {
	path := tempFile()
	db, err := storage.New(path, 200)
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(path)

	eb, err := New(db, time.Second*5, time.Minute*5)
	if err != nil {
		t.Fatal(err)
	}

	firstEventID := eb.Publish(events.EventNamespaceCreated{
		NamespaceID: "test_namespace_1",
	})
	secondEventID := eb.Publish(events.EventNamespaceCreated{
		NamespaceID: "test_namespace_2",
	})
	thirdEventID := eb.Publish(events.EventNamespaceCreated{
		NamespaceID: "test_namespace_3",
	})
	eb.Publish(events.EventNamespaceCreated{
		NamespaceID: "test_namespace_4",
	})
	eb.Publish(events.EventNamespaceCreated{
		NamespaceID: "test_namespace_5",
	})

	events := eb.GetAll(false)
	event1 := <-events
	event2 := <-events
	event3 := <-events

	if event1.ID != firstEventID {
		t.Errorf("published event id and new event id do no match; published %d; new %d",
			event1.ID, firstEventID)
	}

	if event2.ID != secondEventID {
		t.Errorf("published event id and new event id do no match; published %d; new %d",
			event2.ID, secondEventID)
	}

	if event3.ID != thirdEventID {
		t.Errorf("published event id and new event id do no match; published %d; new %d",
			event3.ID, thirdEventID)
	}
}

func TestGetAllOffset(t *testing.T) {
	path := tempFile()
	db, err := storage.New(path, 200)
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(path)

	eb, err := New(db, time.Second*5, time.Minute*5)
	if err != nil {
		t.Fatal(err)
	}

	eventIDsList := []int64{}
	for i := 0; i < 20; i++ {
		id := eb.Publish(events.EventNamespaceCreated{
			NamespaceID: "test_namespace",
		})
		eventIDsList = append(eventIDsList, id)
	}

	events := eb.GetAll(false)

	count := 0
	for event := range events {
		if event.ID != eventIDsList[count] {
			t.Errorf("published event id and new event id do no match; published %d; new %d",
				event.ID, eventIDsList[count])
		}

		count++
	}
}

func TestGetAllReverse(t *testing.T) {
	path := tempFile()
	db, err := storage.New(path, 200)
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(path)

	eb, err := New(db, time.Second*5, time.Minute*5)
	if err != nil {
		t.Fatal(err)
	}

	eb.Publish(events.EventNamespaceCreated{
		NamespaceID: "test_namespace_1",
	})
	eb.Publish(events.EventNamespaceCreated{
		NamespaceID: "test_namespace_2",
	})
	thirdEventID := eb.Publish(events.EventNamespaceCreated{
		NamespaceID: "test_namespace_3",
	})
	fourthEventID := eb.Publish(events.EventNamespaceCreated{
		NamespaceID: "test_namespace_4",
	})
	fifthEventID := eb.Publish(events.EventNamespaceCreated{
		NamespaceID: "test_namespace_5",
	})

	events := eb.GetAll(true)
	event1 := <-events
	event2 := <-events
	event3 := <-events

	if event1.ID != fifthEventID {
		t.Errorf("published event id and new event id do no match; published %d; new %d",
			event1.ID, fifthEventID)
	}

	if event2.ID != fourthEventID {
		t.Errorf("published event id and new event id do no match; published %d; new %d",
			event2.ID, fourthEventID)
	}

	if event3.ID != thirdEventID {
		t.Errorf("published event id and new event id do no match; published %d; new %d",
			event3.ID, thirdEventID)
	}
}

func TestGetAllReverseOffset(t *testing.T) {
	path := tempFile()
	db, err := storage.New(path, 200)
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(path)

	eb, err := New(db, time.Second*5, time.Minute*5)
	if err != nil {
		t.Fatal(err)
	}

	eventIDsList := []int64{}
	for i := 0; i < 20; i++ {
		id := eb.Publish(events.EventNamespaceCreated{
			NamespaceID: "test_namespace_5",
		})
		eventIDsList = append(eventIDsList, id)
	}

	events := eb.GetAll(true)

	count := 19
	for event := range events {
		if event.ID != eventIDsList[count] {
			t.Errorf("published event id and new event id do no match; published %d; new %d",
				event.ID, eventIDsList[count])
		}

		count--
	}
}

func TestPruneEvents(t *testing.T) {
	path := tempFile()
	db, err := storage.New(path, 200)
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(path)

	eb, err := New(db, time.Millisecond*1, time.Minute*5)
	if err != nil {
		t.Fatal(err)
	}

	id1 := eb.Publish(events.EventNamespaceCreated{
		NamespaceID: "test_namespace_1",
	})
	eb.Publish(events.EventNamespaceCreated{
		NamespaceID: "test_namespace_2",
	})
	eb.Publish(events.EventNamespaceCreated{
		NamespaceID: "test_namespace_3",
	})

	time.Sleep(time.Millisecond * 10)

	eb.pruneEvents()

	id4 := eb.Publish(events.EventNamespaceCreated{
		NamespaceID: "test_namespace_4",
	})
	eb.Publish(events.EventNamespaceCreated{
		NamespaceID: "test_namespace_5",
	})

	storedEvent, err := eb.Get(id4)
	if err != nil {
		t.Fatal(err)
	}

	if storedEvent.ID != id4 {
		t.Errorf("published event id and new event id do no match; published %d; new %d",
			storedEvent.ID, id4)
	}

	storedEvent, err = eb.Get(id1)
	if err != ErrEventNotFound {
		t.Errorf("first event exists, when it should have been pruned; published %d; new %d",
			storedEvent.ID, id1)
		return
	}
}
