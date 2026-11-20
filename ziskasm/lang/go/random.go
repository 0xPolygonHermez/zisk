package zkvm

// Host randomness (zkvm_random.h) and the diagnostic log (write_log of zkvm_io.h).

import "unsafe"

// RandomU64 returns 64 random bits. They come from the seed the host provides with
// each execution, so the author of the input cannot predict them, but they are
// free witness values that carry no attestation: for a fixed input, the output
// must be the same for every value they can take.
func RandomU64() uint64 {
	return randomU64()
}

// WriteLog writes text as diagnostic output. It is not part of the public output
// and does not change the execution or its proof, which sees one no-op step; the
// emulator prints it only when asked to (ziskemu --write-log).
func WriteLog(text string) {
	writeLog(unsafe.Pointer(unsafe.StringData(text)), uintptr(len(text)))
}
