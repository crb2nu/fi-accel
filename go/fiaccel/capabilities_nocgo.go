//go:build !cgo

package fiaccel

func runtimeCapabilities() CapabilitySet {
	return CapabilitySet{
		Tokenizer:       true,
		SIMDJSON:        false,
		Transport:       false,
		Eventlog:        false,
		MarkdownHelpers: true,
		ChunkerCode:     false,
		ChunkerMarkdown: false,
		ChunkerText:     false,
		FFIAvailable:    false,
	}
}
