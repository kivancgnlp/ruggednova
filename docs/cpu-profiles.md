# CPU profiles and the `origin` attribute

RuggedNova targets the base Data General Nova ISA as its correctness baseline, and
optionally accepts Rolm-family extensions when a diagnostic or program needs them.
The choice is made once at start-up via a CPU profile, and drives which instruction
rows the identifier loads out of the XML tables in `Data/Instruction_Informations/`.

## The `origin` attribute

Every `<instruction>` row in the XML files carries an `origin` attribute naming the
earliest CPU that implements the opcode. Recognized values today:

| `origin` | Meaning |
|---|---|
| `Nova` | Base 1969 Data General Nova ISA (Nova, SuperNova, Nova 1200). |
| `Rolm` | Post-Nova extension available on Rolm 1602 or later (1602, 1602A, 1602B, 1603, 1603A, 1664, 1666, 1666B, 1666D). |

### `Nova`

Covers the instructions documented in the original 1969 Nova programmer's reference:

- 8 ALC functions: `ADD`, `SUB`, `ADC`, `NEG`, `COM`, `MOV`, `INC`, `AND`
- 4 non-AC memory-reference instructions: `JMP`, `JSR`, `ISZ`, `DSZ`
- 2 AC memory-reference instructions: `LDA`, `STA`
- 7 base I/O opcodes: `NIO`, `DIA`, `DIB`, `DIC`, `DOA`, `DOB`, `DOC`
- 4 base I/O skip forms: `SKPBN`, `SKPBZ`, `SKPDN`, `SKPDZ`
- 7 base status-control opcodes: `INTEN`, `INTDS`, `INTA`, `MSKO`, `READS`, `IORST`, `HALT`

Total: 32 rows across all XML files.

### `Rolm`

Covers every post-Nova opcode currently in the XML tables: hardware stack, byte
manipulation, bit manipulation, file processing, extended-memory forms, floating-
decimal arithmetic, MMU/resource-management, `LEF`, `TRAP`, `MUL`/`DIV`, `JMPE`,
`PJSE`, and the ION/power-fail skip forms.

Rolm parts are upward-compatible along `1601 → 1602 → 1602A → 1602B → 1603 →
1603A → 1664 → 1666 → 1666B → 1666D`, with MSE-14/25 and HAWK-32 as later
Rolm designs branching from that lineage. Any instruction that appeared on
the Rolm 1602 also runs on the 1603, 1664, and 1666-series. `origin="Rolm"`
therefore means "any Rolm >= 1602", not "1602 only".

Total: 135 rows across all XML files.

## CPU profile selection

Profiles are selected with `--cpu <name>`:

| Profile | Loads rows with |
|---|---|
| `--cpu Nova` | `origin="Nova"` only |
| `--cpu Rolm` | `origin="Nova"` or `origin="Rolm"` |

The identifier applies this filter once, inside `InstructionIdentifier::new`, before
the decode loop runs. The hot per-word decode path is unchanged and stays
profile-agnostic.

## Why this shape

Two design points worth calling out:

**`origin` is a scalar, checked via CPU inheritance.** Because the Nova → Rolm and
intra-Rolm chains are strictly additive, one attribute per row plus a rank check in
code is enough. There is no need for a set-valued `origin` or per-CPU duplicated
rows.

**`origin` and `base_type` are orthogonal.** `base_type` describes the encoding shape
(how the decoder unpacks fields for that word), while `origin` describes which CPU
implements the opcode. Two rows can share `base_type` but differ in `origin` (e.g.
plain ALC vs. `LEF`, both `ALC_W_NLNS`), and vice versa. Keep them separate.

## What this fixes today

The `LEF` row uses `match_value="8008" match_mask="E0FF"`, which structurally
aliases the base-Nova encoding of `MOV# 0,0` (`0x8208` masked by `0xE0FF` = `0x8008`).
Before profile filtering, this collision fired the identifier's
`may_conflicting_instructions_log` and required a hand-coded ALC vs. LEF tie-break.

With `origin="Rolm"` on `LEF` and `--cpu Nova` selected, the `LEF` row is filtered
out at load time and `0x8208` decodes unambiguously as `MOV# 0,0` — the base-Nova
"test AC0 without side effect" idiom used by the 1969 Nova Logic Test at addresses
`0x0122`, `0x01ed`, `0x01fa`, `0x022d`, and `0x0248`.

## Future work

- **Adding a finer Rolm profile.** If we ever need to reject newer instructions
  on an older Rolm profile (e.g. `--cpu Rolm1602`), split the current `Rolm`
  bucket into `Rolm1602` / `Rolm1603` / `Rolm1664` / `Rolm1666` and update the
  rank table. Existing behavior under `--cpu Rolm` is preserved as long as
  `--cpu Rolm` continues to mean "any Rolm". The Novas Are Forever ROLM archive
  (linked in Sources below) has the per-model Programmer's Reference Manuals
  needed to attribute each currently-`Rolm`-tagged row to its introducing model.
- **Auto-selecting the profile from the tape name.** The default diagnostic image
  hardcoded in `main.rs` (`095-000005-01__Nova_Logic_Test__1969.ab`) is a base Nova
  test; auto-selecting `Nova` for that name and letting `--cpu` override would make
  the test matrix self-documenting.
- **Strict origin CI check.** Once every row is annotated, add a `--strict-origin`
  flag that errors on any un-annotated row, and gate CI on it.

## Sources

### Base Data General Nova

- [Data General Nova base instruction reference (rcn.com)](http://users.rcn.com/crfriend/museum/doco/DG/Nova/base-instr.html)
- [Data General Nova (Wikipedia)](https://en.wikipedia.org/wiki/Data_General_Nova) — No-Load and Carry-Control semantics
- [Data General Nova 3 Datapro report (bitsavers)](http://bitsavers.informatik.uni-stuttgart.de/pdf/datapro/datapro_reports_70s-90s/DG/M11-304-10_7909_DG_Nova3.pdf) — Nova 3 additions

### Rolm ruggedized Nova family

- [Novas Are Forever — ROLM documentation archive](https://novasareforever.org/archives/documentation/rolm) — index of Rolm CPU manuals, diagnostics, release notices, and product catalogs
- [ROLM Model 1602 User's Manual (1974)](https://novasareforever.org/user/archive/public/docs/rolm/493_processors/493-105029-00__ROLM_Model_1602_Users_Manual__1974.pdf)
- [ROLM Model 5605 / 1602A / 1602B / 1626 / 1650 Processor Programmer's Reference Manual (1981)](https://novasareforever.org/user/archive/public/docs/rolm/493_processors/493-150053-01__ROLM_Model_5605_1602A_1602B_1626_1650_Processor_Programmers_Reference_Manual__1981.pdf)
- [ROLM Model 1603A Processor Programmer's Reference Manual (1977)](https://novasareforever.org/user/archive/public/docs/rolm/493_processors/493-103200-00__ROLM_Model_1603A_Processor_Programmers_Reference_Manual__1977.pdf)
- [ROLM Model 1664 AN/UYK-28 Processor Programmer's Reference Manual (1975)](https://novasareforever.org/user/archive/public/docs/rolm/493_processors/493-101400-00__ROLM_Model_1664_AN-UYK-28_Processor_Programmers_Reference_Manual__1975.pdf)
- [ROLM Model 1666 and 1666D Processor Programmer's Reference Manual (1977-1983)](https://novasareforever.org/user/archive/public/docs/rolm/493_processors/493-150055-02__ROLM_Model_1666_and_1666D_Processor_Programmers_Reference_Manual__1977-1983.pdf)
- [ROLM Model 1666B Processor Programmer's Reference Manual (1981-1983)](https://novasareforever.org/user/archive/public/docs/rolm/493_processors/493-150084-00__ROLM_Model_1666B_Processor_Programmers_Reference_manual__1981-1983.pdf)
- [ROLM Model 1602B / 1650 Quick Reference Guide (1978)](https://novasareforever.org/user/archive/public/docs/rolm/493_processors/ROLM_Model_1602B-1650_Quick_Reference_Guide__1978.pdf)
- [NRC ML20004F950 (Reactor Safety Systems using Hardened Computers)](https://www.nrc.gov/docs/ML2000/ML20004F950.pdf) — Rolm 1602/1664/1666 lineage and instruction-set duplication policy
- [ROLM Corporation (Wikipedia)](https://en.wikipedia.org/wiki/ROLM) — Rolm ruggedized-Nova product history
