//go:build cgo

package fiaccel

func runtimeCapabilities() CapabilitySet {
	return CapabilitySet{
		Tokenizer:       true,
		SIMDJSON:        true,
		Transport:       true,
		Eventlog:        true,
		MarkdownHelpers: true,
		ChunkerCode:     true,
		ChunkerMarkdown: true,
		ChunkerText:     true,
		FFIAvailable:    true,
	}
}
