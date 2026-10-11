// Virtual-base member access ABI: cppgm++ threads a companion __pvbptr
// pointer parameter for every reference/pointer to a type with a virtual
// base, instead of reading the virtual-base offset from the object's vtable
// the way the Itanium ABI (and clang) does.  The companion-pointer scheme is
// self-consistent for direct call chains, so this reducer runs correctly;
// it documents the ABI divergence behind the clang self-host --emit-ast
// crash, where the companion pointer is lost at some call boundary and left
// garbage.  See docs/v4/moves.md.
//
//   cppgm++ -O3 get_width  =>  mov rax,[rsi+0x8]; ret            (companion ptr)
//   clang   -O3 get_width  =>  mov rax,[rdi]; mov rax,[rax-0x18]; mov rax,[rdi+rax+0x8]; ret
struct ios_base { long width_; long fill_; virtual ~ios_base() {} };
struct basic_ios : public ios_base { void* rdbuf_; };
struct basic_ostream : virtual public basic_ios { long extra_; };
long get_width(basic_ostream& os) { return os.width_; }
