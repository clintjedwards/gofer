package eventbus

import (
	"io/ioutil"
	"os"
	"testing"
	"time"

	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/storage"
	"github.com/clintjedwards/gofer/internal/storage/bolt"
)

func mustOpenDB() storage.Engine {
	path := tempfile()
	db, err := bolt.New(path, 1000)
	if err != nil {
		panic(err)
	}

	return &db
}

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

func TestPublish(t *testing.T) {
	db := mustOpenDB()

	eb, err := New(db, time.Second*5, time.Minute*5)
	if err != nil {
		t.Fatal(err)
	}

	newEvent := models.NewEventStartedRun(models.Run{})

	eb.Publish(newEvent)

	storedEvent, err := eb.Get(newEvent.GetID())
	if err != nil {
		t.Fatal(err)
	}

	if storedEvent.GetID() != newEvent.GetID() {
		t.Errorf("published event id and new event id do no match; published %d; new %d",
			storedEvent.GetID(), newEvent.GetID())
	}
}

func TestSubscribe(t *testing.T) {
	db := mustOpenDB()

	eb, err := New(db, time.Minute*5, time.Minute*5)
	if err != nil {
		t.Fatal(err)
	}

	sub, err := eb.Subscribe(models.StartedRunEvent)
	if err != nil {
		t.Fatal(err)
	}

	eb.Publish(models.NewEventStartedRun(models.Run{}))
	eb.Publish(models.NewEventStartedRun(models.Run{}))
	thirdEvent := models.NewEventStartedRun(models.Run{})
	eb.Publish(thirdEvent)

	<-sub.Events
	<-sub.Events
	three := <-sub.Events
	if three.GetID() != thirdEvent.GetID() {
		t.Errorf("published event id and new event id do no match; published %d; new %d",
			three.GetID(), thirdEvent.GetID())
	}
}

func TestGetAll(t *testing.T) {
	db := mustOpenDB()

	eb, err := New(db, time.Second*5, time.Minute*5)
	if err != nil {
		t.Fatal(err)
	}

	firstEvent := models.NewEventStartedRun(models.Run{})
	secondEvent := models.NewEventStartedRun(models.Run{})
	thirdEvent := models.NewEventStartedRun(models.Run{})
	fourthEvent := models.NewEventStartedRun(models.Run{})
	fifthEvent := models.NewEventStartedRun(models.Run{})

	eb.Publish(firstEvent)
	eb.Publish(secondEvent)
	eb.Publish(thirdEvent)
	eb.Publish(fourthEvent)
	eb.Publish(fifthEvent)

	events := eb.GetAll(false)
	event1 := <-events
	event2 := <-events
	event3 := <-events

	if event1.GetID() != firstEvent.GetID() {
		t.Errorf("published event id and new event id do no match; published %d; new %d",
			event1.GetID(), firstEvent.GetID())
	}

	if event2.GetID() != secondEvent.GetID() {
		t.Errorf("published event id and new event id do no match; published %d; new %d",
			event2.GetID(), secondEvent.GetID())
	}

	if event3.GetID() != thirdEvent.GetID() {
		t.Errorf("published event id and new event id do no match; published %d; new %d",
			event3.GetID(), thirdEvent.GetID())
	}
}

func TestGetAllOffset(t *testing.T) {
	db := mustOpenDB()

	eb, err := New(db, time.Second*5, time.Minute*5)
	if err != nil {
		t.Fatal(err)
	}

	eventList := []models.Event{}
	for i := 0; i < 20; i++ {
		newEvent := models.NewEventStartedRun(models.Run{})
		eb.Publish(newEvent)
		eventList = append(eventList, newEvent)
	}

	events := eb.GetAll(false)

	count := 0
	for event := range events {
		if event.GetID() != eventList[count].GetID() {
			t.Errorf("published event id and new event id do no match; published %d; new %d",
				event.GetID(), eventList[count].GetID())
		}

		count++
	}
}

func TestGetAllReverse(t *testing.T) {
	db := mustOpenDB()

	eb, err := New(db, time.Second*5, time.Minute*5)
	if err != nil {
		t.Fatal(err)
	}

	firstEvent := models.NewEventStartedRun(models.Run{})
	secondEvent := models.NewEventStartedRun(models.Run{})
	thirdEvent := models.NewEventStartedRun(models.Run{})
	fourthEvent := models.NewEventStartedRun(models.Run{})
	fifthEvent := models.NewEventStartedRun(models.Run{})

	eb.Publish(firstEvent)
	eb.Publish(secondEvent)
	eb.Publish(thirdEvent)
	eb.Publish(fourthEvent)
	eb.Publish(fifthEvent)

	events := eb.GetAll(true)
	event1 := <-events
	event2 := <-events
	event3 := <-events

	if event1.GetID() != fifthEvent.GetID() {
		t.Errorf("published event id and new event id do no match; published %d; new %d",
			event1.GetID(), firstEvent.GetID())
	}

	if event2.GetID() != fourthEvent.GetID() {
		t.Errorf("published event id and new event id do no match; published %d; new %d",
			event2.GetID(), secondEvent.GetID())
	}

	if event3.GetID() != thirdEvent.GetID() {
		t.Errorf("published event id and new event id do no match; published %d; new %d",
			event3.GetID(), thirdEvent.GetID())
	}
}

func TestGetAllReverseOffset(t *testing.T) {
	db := mustOpenDB()

	eb, err := New(db, time.Second*5, time.Minute*5)
	if err != nil {
		t.Fatal(err)
	}

	eventList := []models.Event{}
	for i := 0; i < 20; i++ {
		newEvent := models.NewEventStartedRun(models.Run{})
		eb.Publish(newEvent)
		eventList = append(eventList, newEvent)
	}

	events := eb.GetAll(true)

	count := 19
	for event := range events {
		if event.GetID() != eventList[count].GetID() {
			t.Errorf("published event id and new event id do no match; published %d; new %d",
				event.GetID(), eventList[count].GetID())
		}

		count--
	}
}

func TestPruneEvents(t *testing.T) {
	db := mustOpenDB()

	eb, err := New(db, time.Millisecond*1, time.Minute*5)
	if err != nil {
		t.Fatal(err)
	}

	firstEvent := models.NewEventStartedRun(models.Run{})
	secondEvent := models.NewEventStartedRun(models.Run{})
	thirdEvent := models.NewEventStartedRun(models.Run{})

	eb.Publish(firstEvent)
	eb.Publish(secondEvent)
	eb.Publish(thirdEvent)

	time.Sleep(time.Millisecond * 10)

	eb.pruneEvents()

	fourthEvent := models.NewEventStartedRun(models.Run{})
	fifthEvent := models.NewEventStartedRun(models.Run{})
	eb.Publish(fourthEvent)
	eb.Publish(fifthEvent)

	storedEvent, err := eb.Get(fourthEvent.GetID())
	if err != nil {
		t.Fatal(err)
	}

	if storedEvent.GetID() != fourthEvent.GetID() {
		t.Errorf("published event id and new event id do no match; published %d; new %d",
			storedEvent.GetID(), fourthEvent.GetID())
	}

	storedEvent, err = eb.Get(firstEvent.GetID())
	if err != ErrEventNotFound {
		t.Errorf("first event exists, when it should have been pruned; published %d; new %d",
			storedEvent.GetID(), fourthEvent.GetID())
		return
	}
}
