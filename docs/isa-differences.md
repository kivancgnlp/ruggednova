# ISA Differences: Data General Nova → ROLM 1602 → 1606 → 1664 → 1666

Reference document for the RuggedNova emulator. Every architectural claim below is grounded in a primary source: the original ROLM Programmer's Reference Manuals for the 1602, 1664, and 1666 (also covering 1606 as "1666 minus FPU"). File-level citations are collected at the end. Where a claim is derived from cross-manual comparison rather than a single manual, that is noted inline.

The chain is not strictly linear. The 1606 is functionally a 1666 with the Floating-Point Processor removed, and the 1666 itself replaces most of the 1664's Executive-Mode instructions with a Resource-Management instruction set. The right mental model is:

```
Nova (baseline)
  └── ROLM 1602/1602A/1602B/1626/1650   (Nova + Rolm stack/interrupt + EAU-style extensions)
        └── ROLM 1664                    (adds FPU, Executive/User mode, SL/FP registers, 5-word RTFNI)
              ├── ROLM 1666              (drops most 1664 XMD ops; adds RMU + MSR; keeps FPU)
              │     └── ROLM 1606        (identical to 1666, minus FPU)
              └── ROLM 1666B / 1666D     (later 1666 variants; 1666D adds nuclear hardening options)
```

---

## 1. Baseline: Data General Nova instruction set

All ROLM processors in this family execute the Data General Nova instruction set as a strict superset. The Nova baseline defines:

- Four 16-bit accumulators AC0-AC3, a carry bit, and a 15-bit program counter.
- Three instruction formats, distinguished by bits 0-2:
  - **Memory-reference** (LDA, STA, ISZ, DSZ, JMP, JSR): bits 0-2 = `000..010`; encodes AC, indirect flag, index mode (page zero, PC-relative, AC2-relative, AC3-relative), and 8-bit displacement.
  - **ALC (Arithmetic/Logic/Complement)**: bits 0-2 = `1xx`; encodes source AC, destination AC, function (COM/NEG/MOV/INC/ADC/SUB/ADD/AND), shift (L/R/S), carry-in modifier (Z/O/C), no-load, and 3-bit skip.
  - **I/O**: bits 0-2 = `011`; encodes transfer (NIO/DIA/DOA/DIB/DOB/DIC/DOC/SKP), control (start/clear/pulse), and 6-bit device code.
- 15-bit direct addressing with indirection chains permitted.
- Interrupt facility with a single ION flag, INTA/INTDS/MSKO/IORST on device code 77 (CPU).
- No stack, no floating point, no memory protection, no privileged instructions.

Every ROLM CPU below inherits this encoding verbatim, then extends it.

---

## 2. Feature matrix at a glance

Legend: ✓ present, ✗ absent, • present but with the caveat noted in the profile-specific sections below.

| Feature | Nova | 1602 | 1606 | 1664 | 1666 |
|---|---|---|---|---|---|
| Nova AC0-AC3 + carry + 15-bit PC | ✓ | ✓ | ✓ | ✓ | ✓ |
| Nova memory-reference / ALC / I/O formats | ✓ | ✓ | ✓ | ✓ | ✓ |
| ALC no-load + no-skip combination reserved as extension | ✗ | • | ✓ | • | ✓ |
| Hardware Multiply / Divide (EAU-style) | ✗ | • | ✓ | ✓ | ✓ |
| Byte-manipulation instructions | ✗ | ✗ | ✓ | ✓ | ✓ |
| Stack Pointer (SP) | ✗ | ✓ | ✓ | ✓ | ✓ |
| Stack Limit register (SL) with WSL/RSL | ✗ | ✗ | ✓ | ✓ | ✓ |
| Frame Pointer (FP) with WFP/RFP | ✗ | ✗ | ✓ | ✓ | ✓ |
| SAVE instruction (7-word frame) | ✗ | ✗ | ✓ | ✓ | ✓ |
| PJS / PJSE calling convention | ✗ | ✗ | ✓ | ✓ | ✓ |
| Simple-branch interrupt (Location 0 return) | ✓ | ✓ | ✓ | ✓ | ✓ |
| Branch-and-Nest interrupt (STIBN/CLIBN) | ✗ | ✓ | ✓ | ✓ | ✓ |
| Branch-and-Nest CMASK OR DMASK before MSKO | ✗ | ✗ | ✓ | ✓ | ✓ |
| INTA response masked to 6 bits (vs 10) | — | 10 | 6 | 6 | 6 |
| RTFNI stack pop width | — | 2 words (MASK, PC) | 5 words | 5 words | 5 words |
| Executive/User mode split | ✗ | ✗ | ✓ | ✓ | ✓ |
| Protection Status Word (PSW) | ✗ | ✗ | ✗ | ✓ | ✗ (replaced by MSR) |
| Map Status Register (MSR) | ✗ | ✗ | ✓ | ✗ | ✓ |
| Resource Management Unit (RMU) + WRMAP/EXMAP/DXMAP/MAPSI | ✗ | ✗ | ✓ | ✗ | ✓ |
| 1664-style Executive Mode instruction set (STEM/CLEM/…) | ✗ | ✗ | ✗ (only EUB, RLAF retained) | ✓ | ✗ (only EUB, RLAF retained) |
| Stack overflow trap auto-switches to XMD and clears ION | — | ✗ (no XMD) | ✗ | ✓ | ✗ |
| Floating-Point Processor (FAC0-FAC7, sel. precision) | ✗ | ✗ | ✗ | ✓ | ✓ |
| Interrupt Stack Pointer (ISP) / Limit (ISL) | ✗ | ✗ | ✓ | ✓ | ✓ |
| Device code 2 reserved for remote memory chassis | ✗ | ✗ | ✓ | ✗ | ✓ |

---

## 3. Nova → ROLM 1602

The 1602 is a Nova-compatible CPU that adds a small, well-defined set of extensions on top of the base Nova ISA. The 1602 User's Manual (493-105029-00, 1974) and the 1602A/1602B/1626/1650 Programmer's Reference (493-150053-01, 1981) are the canonical sources.

### 3.1 Additions over Nova

- **Stack Pointer (SP)** with WSP/RSP instructions. A software stack lives in main memory; SP is a normal word in memory (there is no separate SL register on the 1602 — the overflow floor is hardwired to location 420 octal). Overflow traps through locations 44/45 with ION cleared.
- **Branch-and-Nest interrupt facility** (STIBN/CLIBN, N-bit in the ISAW word). When enabled and ISAW bit 0 = 1, an interrupt pushes the previous MASK (from location 5) and the previous PC onto the stack, then jumps to ISA.
- **RTFNI (Return From Nested Interrupt)** at opcode 060301 octal / 0x60C1. On the 1602, RTFNI pops exactly **two** words from the stack: the previous MASK (stored into location 5 and reissued as MSKO) and the previous PC. ION is left unchanged.
- **Auto-index locations** at 100020-100037 octal (used by indirect addressing to increment/decrement automatically).
- **EAU-style extensions** on some models (multiply/divide) reachable via I/O instructions with device code 77 and specific control fields. The 1601 uses this code for its own extended instructions; the 1602 supports MUL/DIV but not the 1601 shift set.

### 3.2 Constraints for later compatibility

- The 1602 uses the ALC no-load + no-skip encoding as a real (non-trapping) instruction. Later CPUs (1606/1666) reserve exactly this encoding for new instruction formats — programs targeting 1666+ must not use no-load + no-skip together.
- I/O device codes 00 and 01 are ordinary I/O on the 1602; the 1666 reassigns them as extended instruction escapes.
- Programs that use the stack must (on later CPUs) initialize the Stack Limit register to 420₈ to reproduce 1602-compatible overflow behavior.
- Branch-and-Nest programs must initialize the System Data Table Pointer at location 3 to run on 1666.

### 3.3 What the 1602 does *not* have

No Executive/User mode, no Protection Status Word, no Stack Limit register, no Frame Pointer, no SAVE, no PJS/PJSE, no byte-manipulation instructions, no Floating-Point Processor, no memory map, no Resource Management Unit.

---

## 4. ROLM 1602 → ROLM 1664

The 1664 (AN-UYK-28) Programmer's Reference (493-101400-00, 1975) is a large step up. Section 1-2 summarizes the additions as a hardware Floating-Point Processor, Executive/User modes, an extensive new instruction set, and richer stack management.

### 4.1 Registers added

- **Stack Pointer (SP)** — still present, now backed by a real register plus WSP/RSP.
- **Stack Limit register (SL)** with WSL/RSL. When SP goes below SL, a stack-overflow trap fires; unlike the 1602's hardwired 420₈ floor, SL is programmable.
- **Frame Pointer (FP)** with WFP/RFP. Used with SAVE-based stack frames.
- **Protection Status Word (PSW)** — a hardware-managed word that carries the current mode (Executive/User = XMD flag), APM state, and related protection bits. Not directly writable in User Mode.
- **Interrupt Stack Pointer (ISP)** and **Interrupt Stack Limit (ISL)**, held in the third and fourth words of the System Data Table pointed to by location 3.
- **Eight 64-bit Floating-Point Accumulators FAC0-FAC7** in the Floating-Point Processor.
- **Interrupt Count (ICNT)** in the second word of the System Data Table.

### 4.2 New instruction classes

- **Byte-manipulation instructions** (section 3-4-2 of the 1664 manual). Individual bytes within words can be addressed and operated on directly.
- **Extended fixed-point arithmetic** (section 3-4-5) including instructions beyond the 1602's ALC set.
- **Floating-point instructions** (section 3-4-6): loads/stores between FACs and memory; single/double/extended precision selectable via FSET3/FSET4; hex guard digit; integerize instructions; FCLE/FDTRP/FETRP for trap control.
- **Stack instructions** (SAVE, POPB, and the stack-register I/O ops). SAVE builds a fixed 7-word stack frame that includes AC0-AC3, carry, overflow, and the return linkage.
- **PJS / PJSE calling convention** used as the standard subroutine entry mechanism; arguments beyond the first two are pushed in reverse order, first two go in AC0/AC1.
- **Executive Mode instruction set** (section 3-4-10) including STEM/CLEM (set/clear Expanded Memory flag), STIBN/CLIBN (Interrupt Branch And Nest), WAIT, RTFNI, EUB (Executive to User Branch), RLAF (Read Last Address File). These are privileged.

### 4.3 Interrupt architecture rewrite

Branch-and-Nest on the 1664 saves PSW, previous SP, and previous SL to temporary registers. If the interrupted program was in User Mode, SP and SL are replaced with ISP and ISL from the System Data Table. CMASK (from location 5) is OR'd with the DMASK at ISA-1 before the MSKO, so the effective interrupt mask combines the software-managed mask with the device's own priority mask — this is different from the 1602, which just uses MASK as-is.

The **RTFNI at 060301 / 0x60C1** now pops five words: CMASK, PC, PSW, SP, SL. It restores XMD from PSW (if returning to User Mode, XMD is cleared), reissues MSKO with the popped CMASK, increments SP by 5, and leaves ION unchanged.

### 4.4 Traps and protection

- User Mode is prohibited from executing I/O and privileged instructions; violations trap into Executive Mode.
- Stack overflow, unimplemented-instruction, and DMA-violation traps automatically switch the processor to Executive Mode and clear ION.
- INTA response is still masked to 10 bits.

---

## 5. ROLM 1664 → ROLM 1666

Appendix A of the 1666 Programmer's Reference (493-150055-02, 1977-1983) enumerates the differences under "Qualifications for the 1664":

- "All Executive Mode instructions for the 1664, except for **EUB** and **RLAF**, no longer exist on the 1666. They have been replaced by the **Resource Management** instructions."
- "The **Protection Status Word (PSW)** of the 1664 has been replaced by the **Map Status Register (MSR)**."
- "Most User Mode/Executive Mode concepts that were defined for the 1664 have been replaced by similar constructs involving the Resource Management system."
- "Stack overflow traps and unimplemented instruction traps no longer cause the processor automatically to switch to Executive Mode and disable interrupts."
- "Device code 2 is used for remote memory chassis interrupts."

Concretely, the 1666 changes look like this.

### 5.1 Removed relative to 1664

- The 1664-style Executive Mode instruction set (STEM/CLEM/STIBN/CLIBN/WAIT and companions) — only **EUB** and **RLAF** survive.
- The Protection Status Word as a register.
- Automatic mode-switch + ION-clear on stack-overflow / unimplemented-instruction traps.

### 5.2 Added over 1664

- **Resource Management Unit (RMU)**. Hardware memory mapping and per-page protection, plus I/O protection. Programs interact with it via RMU instructions:
  - **WRMAP** (Write Map), **EXMAP** (Examine Map), **DXMAP** (Delete map entry / Examine), **MAPSI** (Map Sensitive Instruction), and the corresponding trap/return pairs.
  - **ECALL** and **TRAP** for controlled Executive-Mode entry from User Mode.
  - **UJMP** for Executive→User transitions in place of EUB when a mapped transition is required.
- **Map Status Register (MSR)** replaces PSW as the carrier of mode and mapping state.
- INTA response masked to **6 bits** instead of 10 (this also affects Branch-and-Nest).
- The Branch-and-Nest sequence explicitly uses the CMASK-OR-DMASK computation (this was already true on the 1664; the 1666 manual re-emphasizes it as a difference from the 1602).
- **Device code 2** reserved for remote memory chassis interrupts.
- The ALC no-load + no-skip encoding is fully reserved as an extended instruction escape; it is no longer available as a regular ALC.
- I/O device codes 00 and 01 are extended-instruction escapes.

### 5.3 Kept from 1664

- The full 5-word RTFNI (same 060301 / 0x60C1 encoding, same pop shape). Semantically the popped protection word is now MSR rather than PSW.
- SP / SL / FP registers and their WSP/RSP/WSL/RSL/WFP/RFP instructions.
- SAVE, PJS, PJSE.
- Byte-manipulation instructions.
- Full FPU with FAC0-FAC7 and selectable precision.
- ISP / ISL and the System Data Table layout.

---

## 6. ROLM 1666 → ROLM 1606

Section 1.1 of the 1666 manual states this cleanly: *"Everything in this manual also applies to the Model 1606 except for references to the Floating-Point Processing, which is not found in the 1606."*

That is the entire difference. Concretely:

- The 1606 does not implement the Floating-Point instructions in section 3.9 of the 1666 manual.
- The FAC0-FAC7 register file, FPSR, FSET3/FSET4, FCLE/FDTRP/FETRP, and floating-point loads/stores are absent.
- Everything else — RMU, MSR, 5-word RTFNI, SP/SL/FP, byte-manipulation, Executive/User mode, ISP/ISL, device-code-2 reservation, INTA masked to 6 bits, and stack-overflow trap semantics — is identical to the 1666.

The relationship 1606 : 1666 is exactly the same as the relationship 1603 : 1664 (also "same CPU minus FPU"). Later models (1666B, 1666D) add packaging and reliability variants; 1666D M001 covers the nuclear-hardened variant.

---

## 7. RTFNI across the family (worked example)

The single most useful diagnostic difference for an emulator is RTFNI (opcode 060301₈ = 0x60C1, encoding identical everywhere).

| CPU | Words popped | Fields restored | Side effects | ION |
|---|---|---|---|---|
| Nova | — | RTFNI does not exist | — | — |
| 1602 | 2 | previous MASK (→ loc 5, then MSKO), previous PC | none beyond MSKO | unchanged |
| 1664 | 5 | CMASK (→ loc 5, then MSKO), PC, PSW, SP, SL | If PSW says User Mode, clears XMD. Increments SP by 5. | unchanged |
| 1606 | 5 | CMASK (→ loc 5, then MSKO), PC, MSR, SP, SL | Restores mode + mapping from MSR. Increments SP by 5. | unchanged |
| 1666 | 5 | CMASK (→ loc 5, then MSKO), PC, MSR, SP, SL | Restores mode + mapping from MSR. Increments SP by 5. | unchanged |

A profile-aware emulator should therefore:

- On a `Rolm1602` profile: implement the 2-word pop, reject / trap the 5-word pop.
- On `Rolm1664`, `Rolm1606`, `Rolm1666`: implement the 5-word pop; on `Rolm1664`, restore XMD from PSW; on `Rolm1606`/`Rolm1666`, restore from MSR.

The stack-overflow trap side-effect is a second useful discriminator:

- `Rolm1664`: auto-switches to Executive Mode and clears ION.
- `Rolm1606` / `Rolm1666`: does not auto-switch and does not clear ION.
- `Rolm1602`: no XMD to switch; ION cleared as part of the trap sequence.

---

## 8. Suggested profile split for RuggedNova

Consistent with the matrix and the RTFNI table above, four instruction profiles are enough:

- **Rolm1602** — Nova baseline + Rolm stack, WSP/RSP only, 2-word RTFNI, hardwired 420₈ overflow floor, no XMD, no SL/FP/PSW/MSR, no FPU.
- **Rolm1664** — Rolm1602 base + SL/FP registers, SAVE, PJS/PJSE, byte-manipulation, full FPU, Executive/User mode with PSW, 1664-style Executive Mode instruction set (including STEM/CLEM/STIBN/CLIBN/WAIT/EUB/RLAF), 5-word RTFNI restoring PSW, stack-overflow trap auto-switches to Executive Mode + clears ION, INTA masked to 10 bits.
- **Rolm1606** — Rolm1664 registers and stack model, but replace PSW with MSR, drop the 1664 Executive-Mode instruction set (keep only EUB and RLAF), add RMU instructions (WRMAP/EXMAP/DXMAP/MAPSI/ECALL/TRAP/UJMP), 5-word RTFNI restoring MSR, stack-overflow trap does not auto-switch or clear ION, INTA masked to 6 bits, reserve device code 2, no FPU.
- **Rolm1666** — Rolm1606 exactly, plus the Floating-Point instructions in section 3.9 of the 1666 manual and the FAC0-FAC7 / FPSR register file.

The 1603 / 1603A can be modelled as a variant of Rolm1602 (or as a sibling of Rolm1664 minus FPU, depending on which side of the family you compare it against). The 1666B and 1666D are packaging / reliability variants of Rolm1666 with no ISA-level difference relevant to this table.

---

## 9. Sources

All architectural claims trace back to these primary manuals, hosted on the Novas Are Forever archive:

- [ROLM Model 1602 User's Manual, 493-105029-00 (1974)](http://www.novasareforever.org/user/archive/public/docs/rolm/493_processors/493-105029-00__ROLM_Model_1602_Users_Manual__1974.pdf) — 1602 baseline, stack, Branch-and-Nest, RTFNI 2-word pop.
- [ROLM Model 5605 / 1602A / 1602B / 1626 / 1650 Processor Programmer's Reference Manual, 493-150053-01 (1981)](http://www.novasareforever.org/user/archive/public/docs/rolm/493_processors/493-150053-01__ROLM_Model_5605_1602A_1602B_1626_1650_Processor_Programmers_Reference_Manual__1981.pdf) — 1602-family instruction set, WSP/RSP, auto-index locations.
- [ROLM Model 1664 AN-UYK-28 Processor Programmer's Reference Manual, 493-101400-00 (1975)](http://www.novasareforever.org/user/archive/public/docs/rolm/493_processors/493-101400-00__ROLM_Model_1664_AN-UYK-28_Processor_Programmers_Reference_Manual__1975.pdf) — 1664 additions over 1602 (FPU, Executive/User mode, SL/FP, SAVE, PJS/PJSE, byte manipulation, 5-word RTFNI, PSW).
- [ROLM Model 1666 and 1666D Processor Programmer's Reference Manual, 493-150055-02 (1977-1983)](http://www.novasareforever.org/user/archive/public/docs/rolm/493_processors/493-150055-02__ROLM_Model_1666_and_1666D_Processor_Programmers_Reference_Manual__1977-1983.pdf) — 1666 additions over 1664 (RMU, MSR, dropped 1664 XMD ops, changed stack-overflow trap, INTA masked to 6 bits, device code 2), and section 1.1 statement that "everything in this manual also applies to the Model 1606 except for references to the Floating-Point Processing".
- [ROLM Model 1666B Processor Programmer's Reference Manual, 493-150084-00 (1981-1983)](http://www.novasareforever.org/user/archive/public/docs/rolm/493_processors/493-150084-00__ROLM_Model_1666B_Processor_Programmers_Reference_manual__1981-1983.pdf) — 1666B variant, no ISA-level departure from 1666 relevant to this document.

Data General Nova baseline is drawn from the Nova instruction encoding as documented in the Nova / Eclipse Programmer's Reference series (Data General 015-000xxx), and cross-verified against the 1602 and 1664 manuals above, which each treat the Nova ISA as the compatibility floor.
