// Temporary sync map replacement with generics until go 1.18 bakes a bit and one gets released.
package syncmap

import (
	"sync"
)

type Syncmap[Key comparable, Value any] struct {
	m        sync.RWMutex
	internal map[Key]Value
}

func New[Key comparable, Value any]() Syncmap[Key, Value] {
	return Syncmap[Key, Value]{
		internal: map[Key]Value{},
	}
}

func (s *Syncmap[Key, Value]) Get(key Key) (Value, bool) {
	s.m.RLock()
	value, ok := s.internal[key]
	s.m.RUnlock()
	return value, ok
}

func (s *Syncmap[Key, Value]) Set(key Key, value Value) {
	s.m.Lock()
	s.internal[key] = value
	s.m.Unlock()
}

// Swap allows the caller to atomically swap a value utilizing a closure.
// Returning an error in the closure aborts the swap and returns the error to the main swap function.
func (s *Syncmap[Key, Value]) Swap(key Key, fn func(value Value, exists bool) (Value, error)) error {
	s.m.Lock()
	defer s.m.Unlock()
	value, exists := s.internal[key]

	newValue, err := fn(value, exists)
	if err != nil {
		return err
	}

	s.internal[key] = newValue
	return nil
}

func (s *Syncmap[Key, Value]) Delete(key Key) {
	s.m.Lock()
	delete(s.internal, key)
	s.m.Unlock()
}

func (s *Syncmap[Key, Value]) Keys() []Key {
	keys := []Key{}
	s.m.RLock()

	for key := range s.internal {
		keys = append(keys, key)
	}

	s.m.RUnlock()

	return keys
}
