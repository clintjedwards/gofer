package models

func sliceToSet(elements []string) map[string]struct{} {
	elementMap := make(map[string]struct{})
	for _, s := range elements {
		elementMap[s] = struct{}{}
	}
	return elementMap
}
