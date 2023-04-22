package api

import (
	"time"
	"unicode/utf8"

	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/scheduler"
)

// cancelTaskRun calls upon the scheduler to terminate a specific container. The urgency of this request is
// controlled by the force parameter. Normally scheduler will simply send a SIGTERM and wait for a
// graceful exit and on force they will instead send a SIGKILL.
// The associated timeout controls how long the containers are waited upon until they are sent a SIGKILL.
func (api *API) cancelTaskRun(taskRun *models.TaskRun, force bool) error {
	timeout := api.config.TaskRunStopTimeout

	if force {
		timeout = time.Millisecond * 500
	}

	containerID := taskContainerID(taskRun.Namespace, taskRun.Pipeline, taskRun.Run, taskRun.ID)

	err := api.scheduler.StopContainer(scheduler.StopContainerRequest{
		ID:      containerID,
		Timeout: timeout,
	})
	if err != nil {
		return err
	}

	return nil
}

// scanWordsWithWhitespace is a split function for a Scanner that returns each
// space-separated word of text. The definition of space is set by unicode.IsSpace.
func scanWordsWithWhitespace(data []byte, atEOF bool) (advance int, token []byte, err error) {
	start := 0

	// Scan until space, marking end of word.
	for width, i := 0, start; i < len(data); i += width {
		var r rune
		r, width = utf8.DecodeRune(data[i:])
		if isSpace(r) {
			return i + width, data[start : i+1], nil
		}
	}

	// If we're at EOF, we have a final, non-empty, non-terminated word. Return it.
	if atEOF && len(data) > start {
		return len(data), data[start:], nil
	}

	// Request more data.
	return start, nil, nil
}

// isSpace reports whether the character is a Unicode white space character.
// We avoid dependency on the unicode package, but check validity of the implementation
// in the tests.
func isSpace(r rune) bool {
	if r <= '\u00FF' {
		// Obvious ASCII ones: \t through \r plus space. Plus two Latin-1 oddballs.
		switch r {
		case ' ', '\t', '\n', '\v', '\f', '\r':
			return true
		case '\u0085', '\u00A0':
			return true
		}
		return false
	}
	// High-valued ones.
	if '\u2000' <= r && r <= '\u200a' {
		return true
	}
	switch r {
	case '\u1680', '\u2028', '\u2029', '\u202f', '\u205f', '\u3000':
		return true
	}
	return false
}
