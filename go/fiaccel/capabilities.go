package fiaccel

// CapabilitySet reports the stable runtime surface exposed by this binding.
type CapabilitySet struct {
	Tokenizer       bool `json:"tokenizer"`
	SIMDJSON        bool `json:"simd_json"`
	Transport       bool `json:"transport"`
	Eventlog        bool `json:"eventlog"`
	ChunkerCode     bool `json:"chunker_code"`
	ChunkerMarkdown bool `json:"chunker_markdown"`
	ChunkerText     bool `json:"chunker_text"`
	FFIAvailable    bool `json:"ffi_available"`
}

// RuntimeCapabilities reports the capabilities exposed by the Go binding.
func RuntimeCapabilities() CapabilitySet {
	return runtimeCapabilities()
}
