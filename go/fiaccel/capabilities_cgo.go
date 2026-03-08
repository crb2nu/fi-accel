//go:build cgo

package fiaccel

func runtimeCapabilities() CapabilitySet {
	return CapabilitySet{
		Tokenizer:       true,
		SIMDJSON:        true,
		Transport:       true,
		ChunkerCode:     false,
		ChunkerMarkdown: false,
		ChunkerText:     false,
		FFIAvailable:    true,
	}
}
