# From machine instructions to an executable

This is the encoding and executable-layout lesson for PA24. Work through it
as part of implementing `lowir2native`; it introduces the byte emitter,
labels, fixups, and image writer that the backend needs. The LowIR reader
and representation come from PA8.

The instruction-encoding and label-patching discussion adapts the original
CPPGM PA9 design notes. It now applies directly to the native backend.

## Start with a small, inspectable executable

A Linux x86-64 executable needs an entry address and a loadable segment.
For an initial example, put a 64-byte ELF header, a 56-byte program header,
and twelve instruction bytes into one read/execute segment. The entry point
is the first instruction, at virtual address `0x400000 + 120`.

The instructions are:

```text
b8 3c 00 00 00       mov eax, 60     ; Linux exit syscall number
bf 2a 00 00 00       mov edi, 42     ; status
0f 05                syscall
```

This teaching example writes the entire 132-byte image. Serialize fields
explicitly in little-endian order; writing a host C++ structure directly
would make file layout depend on host padding and representation.

```cpp
#include <cstdint>
#include <fstream>
#include <vector>

void append_le(std::vector<unsigned char>& bytes, std::uint64_t value,
               unsigned width)
{
    for (unsigned i = 0; i < width; ++i) {
        bytes.push_back(static_cast<unsigned char>(value & 255));
        value >>= 8;
    }
}

int main()
{
    const std::uint64_t base = 0x400000;
    const std::uint64_t code_offset = 64 + 56;
    const std::uint64_t image_size = code_offset + 12;
    std::vector<unsigned char> image = {
        0x7f, 'E', 'L', 'F', 2, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0
    };
    append_le(image, 2, 2);                   // ET_EXEC
    append_le(image, 62, 2);                  // EM_X86_64
    append_le(image, 1, 4);                   // ELF version
    append_le(image, base + code_offset, 8);  // entry address
    append_le(image, 64, 8);                  // program-header offset
    append_le(image, 0, 8);                   // no section-header table
    append_le(image, 0, 4);                   // flags
    append_le(image, 64, 2);                  // ELF-header size
    append_le(image, 56, 2);                  // program-header size
    append_le(image, 1, 2);                   // one program header
    append_le(image, 0, 2);                   // section-header size
    append_le(image, 0, 2);                   // section count
    append_le(image, 0, 2);                   // section-name table index

    append_le(image, 1, 4);                   // PT_LOAD
    append_le(image, 5, 4);                   // PF_R | PF_X
    append_le(image, 0, 8);                   // segment file offset
    append_le(image, base, 8);                // virtual address
    append_le(image, base, 8);                // physical address
    append_le(image, image_size, 8);          // file bytes
    append_le(image, image_size, 8);          // memory bytes
    append_le(image, 0x1000, 8);              // alignment

    const unsigned char code[] = {
        0xb8, 0x3c, 0, 0, 0, 0xbf, 0x2a, 0, 0, 0, 0x0f, 0x05
    };
    image.insert(image.end(), code, code + sizeof(code));
    if (image.size() != image_size) return 1;
    std::ofstream out("exit42", std::ios::binary);
    out.write(reinterpret_cast<const char*>(image.data()), image.size());
    out.close();
    return out ? 0 : 1;
}
```

Compile the example, run it to write `exit42`, then use `chmod +x exit42`.
`readelf -h -l exit42` shows the entry and load segment. On x86-64 Linux,
running the generated file exits with status 42. This is process entry;
there is no caller to return to and no C runtime to initialize it.

Extend the image writer for actual programs: compute segment offsets and
addresses after layout, separate writable data from executable code, and
account for zero-filled storage in memory size. A loadable segment's file
offset and virtual address must agree modulo its alignment. The starter
image has no symbols, relocation table, or section-header table; later
object-writing assignments add the structures their contracts require.

## Encode the forms your backend selects

Represent native registers, immediates, and memory operands explicitly.
Keep instruction selection separate from byte encoding: a selected add,
load, or branch has a defined operand width and addressing form before the
encoder starts. Reuse patterns shared by those forms.

An x86 instruction can contain prefixes, a REX byte, opcode bytes, a ModR/M
byte, a SIB byte, a displacement, and an immediate. The selected form
determines which fields are present. REX.W selects 64-bit operands for many
integer forms; REX.R/X/B extend register fields. Memory encoding must handle
the special base/index encodings for `rsp`/`r12` and `rbp`/`r13`, including
forms that need a SIB byte or an explicit zero displacement.

Start with moves, integer arithmetic, loads/stores, branches, calls, and
returns. Add the floating and other forms required by the assignment's
LowIR families. Preserve the calling convention when assigning registers:
argument and result registers, caller clobbers, callee saves, and stack
alignment are properties of the call boundary. Fixed scratch registers
must not destroy a live value or an indirect-call target.

Use the existing test groups while extending the encoder. In particular,
`strict/100-immediate-move-encoding-boundaries.t`,
`strict/100-fallthrough-jump-encoding.t`, and
`structural/100-indexed-memory-addressing.t` exercise immediate width,
branch layout, and addressing. They are part of the native suite.

## Resolve labels after layout

Assign each block and global a label. While emitting a reference to a label
whose address is not known, write a placeholder and record the patch offset,
target label, addend, width, and relocation kind. Patch after all relevant
sizes and alignments are known. Keep absolute addresses and relative
displacements as distinct kinds.

For a near `jmp` (`E9`) or `call` (`E8`), the signed 32-bit displacement is:

```text
target_address - (displacement_field_address + 4)
```

The base is the address immediately after the instruction. Check the
signed range before writing the four little-endian bytes. RIP-relative
memory operands also use the end of their instruction as their base; an
immediate after the displacement changes that base. Record enough
information to distinguish these layouts.

Fixed-width near branches make an initial layout straightforward. If a
later optimization shortens an instruction, recompute affected offsets
before applying fixups. Keep code, data, and external-symbol fixups distinct
so later separate compilation can emit relocations rather than guessing
an address.

## Inspect each layer

To check a selected instruction's bytes, write it in a small `.s` file with
`.intel_syntax noprefix`, assemble it with `gcc -c`, and inspect the object
with `objdump -d -M intel`. For an executable with no section headers,
inspect its code as a raw binary or examine its entry address in GDB.

Use `readelf -h -l` for the image layout and `readelf -r` for relocations
once object files are introduced. In GDB, `set disassembly-flavor intel`,
`x/20i $pc`, `stepi`, and `info registers` let you connect the emitted bytes
to control flow and values. Compare behavior at each layer before extending
the instruction set further.
