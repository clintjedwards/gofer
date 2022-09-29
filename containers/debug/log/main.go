package main

import (
	"log"
	"os"
	"strings"
	"time"
)

var version = "test"

var corpus = `
The professionals are doomed in all their vainglory to be perpetually embarking
on the Sisyphean task of a unified and integrated Linux ecosystem,
even if it means turning the kernel into a runtime for the BPF virtual machine,
or making a Rube Goldberg machine of build and deployment pipelines,
as appears to be the most recent trend. The hobbyists are doomed to
shout in the void with no one to hear them. In this tragedy the only victor is chaos
and discord itself, which disguises itself as “progress.”

All that is guaranteed is permanent revolution through constant reinvention,
where by revolution we mean running around in circles. The suits and ties
have forgotten what it was to be Yippies, and for their part the Yippies are fools
who are had by ideas, rather than having ideas.
`

func main() {
	header := os.Getenv("LOGS_HEADER")

	log.Println(header)
	log.Println(version)
	splitCorpus := strings.Split(corpus, "\n")
	for _, line := range splitCorpus {
		log.Println(line)
		time.Sleep(time.Second * 2)
	}
}
