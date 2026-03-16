package fiaccel

import "encoding/json"

// ChunkConfig controls chunk sizing and optional metadata inputs.
type ChunkConfig struct {
	MaxTokens     int
	OverlapTokens int
	FilePath      string
	Model         string
}

// Chunk is a semantic chunk emitted by the Rust chunkers.
type Chunk struct {
	ID            string   `json:"id"`
	Content       string   `json:"content"`
	Kind          string   `json:"kind"`
	ChunkType     string   `json:"chunk_type"`
	Language      string   `json:"language"`
	FilePath      *string  `json:"file_path,omitempty"`
	StartLine     int      `json:"start_line"`
	EndLine       int      `json:"end_line"`
	StartColumn   int      `json:"start_column"`
	EndColumn     int      `json:"end_column"`
	TokenCount    int      `json:"token_count"`
	Name          *string  `json:"name,omitempty"`
	ParentName    *string  `json:"parent_name,omitempty"`
	Signature     *string  `json:"signature,omitempty"`
	Docstring     *string  `json:"docstring,omitempty"`
	Imports       []string `json:"imports,omitempty"`
	ParentType    *string  `json:"parent_type,omitempty"`
	SymbolKind    *string  `json:"symbol_kind,omitempty"`
	ContainerName *string  `json:"container_name,omitempty"`
	ContainerKind *string  `json:"container_kind,omitempty"`
	SymbolPath    *string  `json:"symbol_path,omitempty"`
	Calls         []string `json:"calls,omitempty"`
	Definitions   []string `json:"definitions,omitempty"`
}

// ChunkCode chunks source code using the Rust tree-sitter chunker.
func ChunkCode(content, language string, config *ChunkConfig) ([]Chunk, error) {
	raw, err := chunkCode(content, language, normalizedChunkConfig(config))
	if err != nil {
		return nil, err
	}
	return decodeChunks(raw)
}

// ChunkMarkdown chunks markdown content.
func ChunkMarkdown(content string, config *ChunkConfig) ([]Chunk, error) {
	raw, err := chunkMarkdown(content, normalizedChunkConfig(config))
	if err != nil {
		return nil, err
	}
	return decodeChunks(raw)
}

// ChunkText chunks plain text content.
func ChunkText(content string, config *ChunkConfig) ([]Chunk, error) {
	raw, err := chunkText(content, normalizedChunkConfig(config))
	if err != nil {
		return nil, err
	}
	return decodeChunks(raw)
}

// ChunkAuto detects content type from filePath and chunks accordingly.
func ChunkAuto(content, filePath string, config *ChunkConfig) ([]Chunk, error) {
	cfg := normalizedChunkConfig(config)
	cfg.FilePath = filePath
	raw, err := chunkAuto(content, filePath, cfg)
	if err != nil {
		return nil, err
	}
	return decodeChunks(raw)
}

// DetectContentType reports the chunker content type classification for a path.
func DetectContentType(filePath string) (string, error) {
	return detectContentType(filePath)
}

func decodeChunks(raw []byte) ([]Chunk, error) {
	if len(raw) == 0 {
		return nil, nil
	}
	var chunks []Chunk
	if err := json.Unmarshal(raw, &chunks); err != nil {
		return nil, &Error{Code: ErrFailed, Message: err.Error()}
	}
	return chunks, nil
}

func normalizedChunkConfig(config *ChunkConfig) ChunkConfig {
	if config == nil {
		return ChunkConfig{MaxTokens: 512, OverlapTokens: 50}
	}

	normalized := *config
	if normalized.MaxTokens == 0 {
		normalized.MaxTokens = 512
	}
	return normalized
}
