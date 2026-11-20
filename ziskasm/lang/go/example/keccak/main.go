//go:build wasm

// A wasip1 guest building hashes on the raw ZisK precompiles from Go: it hashes
// its whole stdin with Keccak-256 and SHA-256, running the sponge and the
// padding here and every permutation or compression as the precompile
// instruction, and prints both digests, one per line, in hex.
//
// (A guest that just wants the digests calls zkvm.Keccak256 and zkvm.Sha256
// of the parent package instead; this is how an existing hash implementation
// plugs its permutation into the precompile.)
//
// Build (standard Go) and run on ZisK:
//
//	GOOS=wasip1 GOARCH=wasm go build -ldflags='-s -w' -o guest.wasm .
//	ziskemu --elf guest.wasm -i input.bin   # ziskemu built with --features float
package main

import (
	"encoding/hex"
	"fmt"
	"io"
	"os"

	"github.com/0xPolygonHermez/zisk/ziskasm/lang/go/precompiles"
)

func main() {
	data, err := io.ReadAll(os.Stdin)
	if err != nil {
		fmt.Fprintln(os.Stderr, "read:", err)
		os.Exit(2)
	}
	k := keccak256(data)
	s := sha256(data)
	fmt.Printf("%s\n%s\n", hex.EncodeToString(k[:]), hex.EncodeToString(s[:]))
}

// keccakRate is the rate of Keccak-256 in bytes (1600 - 2*256 bits).
const keccakRate = 136

// keccak256 is the Ethereum Keccak-256 (0x01 domain byte): the sponge in Go,
// every permutation the keccak_f precompile.
func keccak256(data []byte) [32]byte {
	var state [25]uint64
	for len(data) >= keccakRate {
		xorIn(&state, data[:keccakRate])
		precompiles.KeccakF(&state)
		data = data[keccakRate:]
	}
	var last [keccakRate]byte
	copy(last[:], data)
	last[len(data)] ^= 0x01
	last[keccakRate-1] ^= 0x80
	xorIn(&state, last[:])
	precompiles.KeccakF(&state)
	var out [32]byte
	for i := range 4 {
		w := state[i]
		for j := range 8 {
			out[8*i+j] = byte(w >> (8 * j))
		}
	}
	return out
}

// xorIn xors a full rate block into the state, little-endian lanes.
func xorIn(state *[25]uint64, block []byte) {
	for i := range keccakRate / 8 {
		state[i] ^= leWord(block[8*i:])
	}
}

// sha256 is SHA-256: padding and chaining in Go, every compression the sha256_f
// precompile.
func sha256(data []byte) [32]byte {
	// The initial hash value, two 32-bit words per precompile word, h0 low.
	state := [4]uint64{
		0x6a09e667 | 0xbb67ae85<<32, 0x3c6ef372 | 0xa54ff53a<<32,
		0x510e527f | 0x9b05688c<<32, 0x1f83d9ab | 0x5be0cd19<<32,
	}
	bitLen := uint64(len(data)) * 8
	for len(data) >= 64 {
		compress(&state, data[:64])
		data = data[64:]
	}
	// Padding: 0x80, zeros, the 64-bit big-endian bit length; one or two blocks.
	var tail [128]byte
	n := copy(tail[:], data)
	tail[n] = 0x80
	blocks := 1
	if n+1+8 > 64 {
		blocks = 2
	}
	end := 64 * blocks
	for i := range 8 {
		tail[end-1-i] = byte(bitLen >> (8 * i))
	}
	for i := range blocks {
		compress(&state, tail[64*i:64*i+64])
	}
	var out [32]byte
	for i := range 8 {
		h := uint32(state[i/2] >> (32 * uint(i%2)))
		out[4*i] = byte(h >> 24)
		out[4*i+1] = byte(h >> 16)
		out[4*i+2] = byte(h >> 8)
		out[4*i+3] = byte(h)
	}
	return out
}

// compress feeds one 64-byte block to the precompile, as 8 little-endian words
// of the raw block bytes.
func compress(state *[4]uint64, block []byte) {
	var words [8]uint64
	for i := range words {
		words[i] = leWord(block[8*i:])
	}
	precompiles.Sha256F(state, &words)
}

func leWord(b []byte) uint64 {
	var w uint64
	for j := range 8 {
		w |= uint64(b[j]) << (8 * j)
	}
	return w
}
