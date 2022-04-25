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
