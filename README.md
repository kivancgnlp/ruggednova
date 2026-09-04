# RuggedNova

A Rust emulator for the Data General Nova family of 16-bit minicomputers, written to run original 1969-1976 diagnostic paper-tape images end to end.

## What it does

RuggedNova takes a 16-bit Nova paper-tape image, loads it into simulated core memory, and executes it instruction by instruction while producing a full disassembly trace. It is aimed at correctness against real diagnostic software rather than raw speed.

Implemented today:

- Memory Reference instructions (LDA, STA, LEF, and the extended-memory variants)
- Arithmetic and Logic Class (ALC) instructions with the full skip-condition and shift-carry field decoding
- Byte and Bit manipulation
- Fixed-point arithmetic
- Logical and shift
- File processing
- Stack manipulation, including frame and stack-limit handling
- Program flow alteration (JMP, JSR, and PC-relative forms)
- Resource management
- I/O and status control, with a minimal I/O device layer and an RTC that can raise interrupts
- Interrupt and trap handling with proper stack switching between user and executive contexts
- A Memory Mapping Unit with executive/user modes, dual instruction/data maps, expanded memory, and MAPSI/MAPSD one-shot map switches

Not implemented:

- Floating-point instructions (section 3.9 of the reference manual is intentionally skipped)

## Repository layout

```
src/
  main.rs                              entry point, loads a paper tape and runs the decoder
  loaders/                             paper-tape image loader
  instruction_identifier/              XML-driven table of instructions, match_value/match_mask lookup
  instruction_decoder/                 per-format decoders and executors
    alc_format_*                       ALC instruction decode, executor, and field parsing
    memory_reference_*                 memory-reference formats with and without accumulator
    io_format_*                        I/O format instructions
    extended_memory_acc_format_*       extended-memory instructions with one extra word
    generic_instruction_format_decoder/ shared handlers for the common instruction shapes
    bit_utils.rs                       get_bits / set_bits helpers
  assembler/                           small built-in assembler (see below)
  virtual_machine/
    mod.rs                             ExecutionContext: registers, flags, stack, interrupts, traps
    memory_mapping_unit/               MMU, MSR, MVR, page maps, protection tracking
    io_device_emulator/                I/O device stubs
    complex_instruction_executer.rs    multi-step instructions
Data/
  Instruction_Informations/*.xml       instruction tables, one XML file per manual chapter
  Diagnostic images/*.ab               original Data General diagnostic paper tapes
```

## Instruction table format

Instructions are not hard-coded. Each mnemonic is described by a row in one of the XML files under `Data/Instruction_Informations/`, grouped by chapter of the reference manual:

```xml
<instruction mnemonic="LDA"  match_value="2000" match_mask="E000" following_word_count="0" base_type="MEM_REF_WITH_ACC"/>
<instruction mnemonic="LDAE" match_value="A008" match_mask="E03F" following_word_count="1" base_type="ALC_W_NLNS"/>
<instruction mnemonic="LEF"  match_value="8008" match_mask="E0FF" following_word_count="1" base_type="ALC_W_NLNS"/>
```

The identifier masks each instruction word by `match_mask` and compares against `match_value`. Where several rows can match a single 16-bit word, the more specific mask wins, and every ambiguous case is recorded in a conflict log so the tables can be audited. Adding an instruction is an XML edit, not a code change.

## Running

Prerequisites: a recent stable Rust toolchain.

```
cargo run --release
```

By default `main.rs` loads `Data/Diagnostic images/095-000005-01__Nova_Logic_Test__1969.ab`, starts execution at `0x40`, runs up to a configured instruction limit, and writes a trace disassembly to `Data/<image-name>_trace.txt`. Two knobs at the top of `main` control the output:

- `linear_disassembler_mode` writes a plain top-to-bottom disassembly instead of an execution trace.
- `generate_trace_disassembly` toggles per-instruction trace lines.

Each trace line shows the current user, IP, raw instruction word, decoded mnemonic and operands, and the full register/flag state after execution. Call-stack depth is rendered as a leading dot prefix so nested subroutines are visible at a glance. A histogram of mnemonic and instruction-class frequencies is printed at the end of the run.

## The built-in assembler

`src/assembler/` contains a small hand-written assembler that reuses the same XML instruction table as the decoder. It is intentionally minimal: enough to build short test programs, encode single instructions for round-trip tests, and drive functional scenarios from `#[test]` code, not a full macro assembler.

Supported today:

- **ALC instructions** with the full operand syntax, for example `INC OR# AC0,AC1 SNR`. Carry control, no-load, shift, and skip fields are parsed and packed into the instruction word.
- **Extended memory instructions taking one accumulator plus an extra word**: `LDFNW`, `LDFNA`, `STTNA`, `STTNX`, for example `LDFNW AC1 0x1234`.
- **Single-accumulator instructions**: `WMSR`, `PSH`.
- **JMP** with direct and PC-relative displacements (`JMP +x10`, `JMP -x8`).
- **No-argument instructions**: `HALT`, `ECALL`, `WRWRD`, `STEM`, `RTFNI`, `EXMAP`, `DXMAP`, `MAPSI`.
- **Two pseudo-ops** for building test images:
  - `DW 0x1234` writes one literal word.
  - `DUPW 0x10` writes N zero words (a simple `.zero N` equivalent).
- **Comments** with `;` to end of line.

Round-trip is the primary correctness check: an assembled instruction is fed back through the decoder and the resulting text is compared to the source line. The tests under `assembler::tests` demonstrate the shape:

```rust
let asm = Assembler::new()?;
let words = asm.assemble_line("LDFNW AC1 0x1234").unwrap();
// words[0] is the opcode + accumulator, words[1] is 0x1234
```

Mnemonics and operand forms outside the sets listed above are not accepted yet. There is no label resolution, no macro layer, and no separate file-level driver; the assembler is used programmatically from Rust code.

## Diagnostic images

The `Data/Diagnostic images/` folder contains original Data General diagnostic paper tapes (Memory Address Test, Nova Logic Test, SuperNova Logic Test, Nova 800/1200/3 Logic Test, Multiply-Divide Test, Real Time Clock Test, MMU Diagnostic, Exerciser, Instruction Timer, Arithmetic Test). These are the primary correctness workload the emulator is developed against.

## Status

This is a personal research project. The instruction set coverage above is what is wired up and used during diagnostic runs; a number of edge cases (some overflow paths in extended-memory arithmetic, a few skip conditions) are marked in the source as still to be verified.

## License

See `COPYRIGHT.md`. All rights are reserved; viewing and forking on GitHub are permitted under GitHub's Terms of Service. No other rights are granted.
