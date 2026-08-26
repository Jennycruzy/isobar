// Command go-tester round-trips the Isobar Scorer WASM ABI through wazero.
package main

import (
	"context"
	"encoding/binary"
	"flag"
	"fmt"
	"math"
	"os"
	"time"

	"github.com/tetratelabs/wazero"
	"github.com/tetratelabs/wazero/api"
)

const breakdownBytes = 6 * 4

type runtimeMode string

const (
	compiler    runtimeMode = "compiler"
	interpreter runtimeMode = "interpreter"
)

type inputBuffer struct {
	ptr  uint32
	size uint32
}

func main() {
	wasmPath := flag.String("wasm", "target/wasm32-unknown-unknown/release/isobar_scorer.wasm", "path to the WASM module")
	runtimeFlag := flag.String("runtime", "both", "runtime configuration: compiler, interpreter, or both")
	repeat := flag.Int("repeat", 1000, "number of repeated ABI calls for bit-identity checking")
	flag.Parse()

	if *repeat < 1 {
		fail("repeat must be at least 1")
	}
	wasm, err := os.ReadFile(*wasmPath)
	if err != nil {
		fail("read WASM: %v", err)
	}

	modes, err := parseModes(*runtimeFlag)
	if err != nil {
		fail("%v", err)
	}
	for _, mode := range modes {
		if err := run(mode, wasm, *repeat); err != nil {
			fail("%s runtime: %v", mode, err)
		}
	}
}

func parseModes(value string) ([]runtimeMode, error) {
	switch value {
	case "compiler":
		return []runtimeMode{compiler}, nil
	case "interpreter":
		return []runtimeMode{interpreter}, nil
	case "both":
		return []runtimeMode{compiler, interpreter}, nil
	default:
		return nil, fmt.Errorf("unknown runtime %q", value)
	}
}

func run(mode runtimeMode, wasm []byte, repeat int) error {
	ctx := context.Background()
	config := wazero.NewRuntimeConfig()
	if mode == interpreter {
		config = wazero.NewRuntimeConfigInterpreter()
	}
	runtime := wazero.NewRuntimeWithConfig(ctx, config)
	defer runtime.Close(ctx)

	started := time.Now()
	module, err := runtime.Instantiate(ctx, wasm)
	if err != nil {
		return fmt.Errorf("instantiate: %w", err)
	}
	loadDuration := time.Since(started)

	for _, name := range []string{"alloc", "dealloc", "rank_answer", "breakdown_answer"} {
		if module.ExportedFunction(name) == nil {
			return fmt.Errorf("missing export %q", name)
		}
	}
	memory := module.Memory()
	if memory == nil {
		return fmt.Errorf("module did not export linear memory")
	}

	alloc := module.ExportedFunction("alloc")
	dealloc := module.ExportedFunction("dealloc")
	rank := module.ExportedFunction("rank_answer")
	breakdown := module.ExportedFunction("breakdown_answer")

	question := []byte("What is the capital of France?")
	truth := []byte("Paris is the capital of France.")
	answer := []byte("Paris is the capital of France.")
	buffers, err := allocateInputs(ctx, memory, alloc, question, truth, answer)
	if err != nil {
		return err
	}
	defer releaseInputs(ctx, dealloc, buffers)

	args := callArgs(buffers)
	firstScore, err := callF32(ctx, rank, args...)
	if err != nil {
		return fmt.Errorf("rank_answer: %w", err)
	}
	if firstScore < 0.75 || firstScore > 1.0 || math.IsNaN(float64(firstScore)) {
		return fmt.Errorf("invalid self score %.9f", firstScore)
	}

	breakdownResult, err := breakdown.Call(ctx, args...)
	if err != nil {
		return fmt.Errorf("breakdown_answer: %w", err)
	}
	if len(breakdownResult) != 1 {
		return fmt.Errorf("breakdown_answer returned %d values", len(breakdownResult))
	}
	breakdownValues, err := readBreakdown(memory, uint32(breakdownResult[0]))
	if err != nil {
		return err
	}
	if math.Float32bits(firstScore) != math.Float32bits(breakdownValues[5]) {
		return fmt.Errorf("rank/breakdown score mismatch: %.9f != %.9f", firstScore, breakdownValues[5])
	}
	if breakdownValues[4] < 0.0 || breakdownValues[4] > 1.0 || breakdownValues[5] < 0.0 || breakdownValues[5] > 1.0 {
		return fmt.Errorf("breakdown scores outside [0,1]: raw=%.9f score=%.9f", breakdownValues[4], breakdownValues[5])
	}

	for iteration := 1; iteration < repeat; iteration++ {
		score, err := callF32(ctx, rank, args...)
		if err != nil {
			return fmt.Errorf("repeat %d: %w", iteration, err)
		}
		if math.Float32bits(score) != math.Float32bits(firstScore) {
			return fmt.Errorf("non-deterministic score at iteration %d", iteration)
		}
	}

	fmt.Printf(
		"runtime=%s load=%s self=%.6f raw=%.6f score=%.6f repeats=%d [pass]\n",
		mode,
		loadDuration.Round(time.Millisecond),
		firstScore,
		breakdownValues[4],
		breakdownValues[5],
		repeat,
	)
	return nil
}

func allocateInputs(
	ctx context.Context,
	memory api.Memory,
	alloc api.Function,
	values ...[]byte,
) ([]inputBuffer, error) {
	result := make([]inputBuffer, len(values))
	for index, value := range values {
		callResult, err := alloc.Call(ctx, uint64(len(value)))
		if err != nil {
			return nil, fmt.Errorf("alloc input %d: %w", index, err)
		}
		if len(callResult) != 1 {
			return nil, fmt.Errorf("alloc input %d returned %d values", index, len(callResult))
		}
		ptr := uint32(callResult[0])
		if ok := memory.Write(ptr, value); !ok {
			return nil, fmt.Errorf("write input %d at %d", index, ptr)
		}
		result[index] = inputBuffer{ptr: ptr, size: uint32(len(value))}
	}
	return result, nil
}

func releaseInputs(ctx context.Context, dealloc api.Function, values []inputBuffer) {
	for _, value := range values {
		_, _ = dealloc.Call(ctx, uint64(value.ptr), uint64(value.size))
	}
}

func callArgs(values []inputBuffer) []uint64 {
	return []uint64{
		uint64(values[0].ptr), uint64(values[0].size),
		uint64(values[1].ptr), uint64(values[1].size),
		uint64(values[2].ptr), uint64(values[2].size),
	}
}

func callF32(ctx context.Context, function api.Function, args ...uint64) (float32, error) {
	result, err := function.Call(ctx, args...)
	if err != nil {
		return 0.0, err
	}
	if len(result) != 1 {
		return 0.0, fmt.Errorf("returned %d values", len(result))
	}
	return math.Float32frombits(uint32(result[0])), nil
}

func readBreakdown(memory api.Memory, ptr uint32) ([6]float32, error) {
	var result [6]float32
	bytes, ok := memory.Read(ptr, breakdownBytes)
	if !ok {
		return result, fmt.Errorf("read breakdown at %d", ptr)
	}
	for index := range result {
		bits := binary.LittleEndian.Uint32(bytes[index*4:])
		result[index] = math.Float32frombits(bits)
	}
	return result, nil
}

func fail(format string, values ...any) {
	fmt.Fprintf(os.Stderr, format+"\n", values...)
	os.Exit(1)
}
