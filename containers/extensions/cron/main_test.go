package main

import (
	"testing"
	"time"
)

func TestMinutesToCheck(t *testing.T) {
	last := time.Date(2026, 1, 1, 12, 0, 0, 0, time.UTC)

	tests := map[string]struct {
		now  time.Time
		want int
	}{
		"same minute":         {last.Add(30 * time.Second), 0},
		"clock went backward": {last.Add(-2 * time.Minute), 0},
		"normal tick":         {last.Add(time.Minute + 5*time.Millisecond), 1},
		"three minute stall":  {last.Add(3*time.Minute + 10*time.Second), 3},
		"at catchup limit":    {last.Add(maxCatchup), 5},
		"one hour jump":       {last.Add(time.Hour), 1},
	}

	for name, tc := range tests {
		t.Run(name, func(t *testing.T) {
			got := minutesToCheck(last, tc.now)
			if len(got) != tc.want {
				t.Fatalf("got %d minutes, want %d: %v", len(got), tc.want, got)
			}
			if len(got) > 0 && !got[len(got)-1].Equal(tc.now.Truncate(time.Minute)) {
				t.Fatalf("last minute %v, want %v", got[len(got)-1], tc.now.Truncate(time.Minute))
			}
		})
	}
}
