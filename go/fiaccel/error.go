package fiaccel

import "fmt"

// Error represents an error from fi-accel FFI.
type Error struct {
	Code    ErrorCode
	Message string
}

// ErrorCode represents the category of error.
type ErrorCode int

const (
	ErrOk          ErrorCode = 0
	ErrInvalidArg  ErrorCode = 1
	ErrFailed      ErrorCode = 2
	ErrNotFound    ErrorCode = 3
	ErrOutOfMemory ErrorCode = 4
)

func (e *Error) Error() string {
	return fmt.Sprintf("fi-accel error %d: %s", e.Code, e.Message)
}

// ErrNotAvailable is returned when CGo is not enabled and the
// pure-Go fallback is used.
var ErrNotAvailable = &Error{Code: ErrFailed, Message: "fi-accel native library not available (CGo disabled)"}
