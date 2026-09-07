//go:build tamago && riscv64

package main

import (
	"fmt"
	_ "tamagotest/tamaboards/zkvm"
	"tamagotest/tamaboards/zkvm/zisk_runtime"
)

func main() {
	// Read two integers using the generic Read function
	_a := zisk_runtime.Read[int64]()
	_b := zisk_runtime.Read[int64]()
	a := float32(_a) + 0.1
	b := float32(_b) + 0.4

	// fmt.Printf("Calculator Test Program\n")
	// fmt.Printf("a = %f, b = %f\n", a, b)
	// fmt.Printf("Addition: %f + %f = %f\n", a, b, a+b)
	fmt.Printf("Subtraction: %f - %f = %f\n", a, b, a-b)
	// fmt.Printf("Multiplication: %f * %f = %f\n", a, b, a*b)
	// fmt.Printf("Division: %f / %f = %f\n", a, b, a/b)
}
