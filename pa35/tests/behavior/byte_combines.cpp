// Byte loads shifted into place and or-ed together (combining them into
// one wider load is legal but stalls when the bytes were stored one by one
// just before): flag bytes packed into a word, two, four and
// eight bytes, u16 halves, a byte stored between the loads, a reversed
// order, three bytes, bytes of two objects, a narrowing back through an
// extension.
#include <cstdio>
#include <cstdint>
#include <cstring>
struct Flags
{
	uint8_t kind, cv, function_cv, ref;
	bool variadic;
	uint8_t pad[3];
	uint16_t lo, hi;
	uint8_t bytes[8];
};
__attribute__((noinline)) uint32_t packed(const Flags& f)
{
	return f.cv | (f.function_cv << 8) | (static_cast<unsigned>(f.ref) << 16) | (static_cast<unsigned>(f.variadic) << 24);
}
__attribute__((noinline)) uint16_t pair(const Flags& f)
{
	return static_cast<uint16_t>(f.kind | (f.cv << 8));
}
__attribute__((noinline)) uint64_t eight(const Flags& f)
{
	uint64_t v = 0;
	for (int i = 0; i < 8; ++i)
		v |= static_cast<uint64_t>(f.bytes[i]) << (8 * i);
	return v;
}
__attribute__((noinline)) uint32_t halves(const Flags& f)
{
	return f.lo | (static_cast<uint32_t>(f.hi) << 16);
}
__attribute__((noinline)) uint32_t stored_between(Flags& f, uint8_t x)
{
	const uint32_t a = f.cv;
	f.function_cv = x;
	return a | (f.function_cv << 8) | (static_cast<unsigned>(f.ref) << 16) | (static_cast<unsigned>(f.variadic) << 24);
}
__attribute__((noinline)) uint32_t reversed(const Flags& f)
{
	return (static_cast<uint32_t>(f.bytes[0]) << 24) | (f.bytes[1] << 16) | (f.bytes[2] << 8) | f.bytes[3];
}
__attribute__((noinline)) uint32_t three(const Flags& f)
{
	return f.bytes[0] | (f.bytes[1] << 8) | (f.bytes[2] << 16);
}
__attribute__((noinline)) uint32_t two_objects(const Flags& f, const Flags& g)
{
	return f.bytes[0] | (g.bytes[1] << 8) | (f.bytes[2] << 16) | (static_cast<uint32_t>(g.bytes[3]) << 24);
}
__attribute__((noinline)) uint32_t mixed(const Flags& f, uint32_t seed)
{
	const uint64_t wide = packed(f);
	return static_cast<uint32_t>(wide) * 0x01000193u + seed;
}
int main()
{
	Flags f;
	std::memset(&f, 0, sizeof f);
	f.kind = 0x11;
	f.cv = 0x82;
	f.function_cv = 0x7f;
	f.ref = 0xc3;
	f.variadic = true;
	f.lo = 0xbeef;
	f.hi = 0x8001;
	for (int i = 0; i < 8; ++i)
		f.bytes[i] = static_cast<uint8_t>(0xf1 - 0x13 * i);
	Flags g = f;
	g.bytes[1] = 0x5a;
	g.bytes[3] = 0xa5;
	std::printf("packed %08x pair %04x eight %016llx halves %08x\n", packed(f), pair(f),
		static_cast<unsigned long long>(eight(f)), halves(f));
	std::printf("stored %08x", stored_between(f, 0x44));
	std::printf(" now %08x\n", packed(f));
	std::printf("reversed %08x three %08x two %08x mixed %08x\n", reversed(f), three(f), two_objects(f, g), mixed(f, 9));
	return 0;
}
