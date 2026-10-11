// 32-bit zero-extensions of values 64-bit operations computed (O1+: the
// operation runs at 32 bits instead, MIR fuse_adds_and_extensions) and of
// narrow parameters copied out of their incoming registers on entry (the
// copy extends): truncated differences and sums used as indexes, masks of
// complemented and negated words, shifted and multiplied sizes, wide loads
// read through their low half, moved u32 and signed char parameters.
#include <cstdio>
#include <cstdint>
struct Table
{
	uint64_t sizes[8];
	uint32_t bits[8];
};
__attribute__((noinline)) uint64_t last_index(const Table& t, uint64_t n, uint64_t k)
{
	const uint32_t i = static_cast<uint32_t>(n - 1);
	const uint32_t j = static_cast<uint32_t>(n + k);
	return (static_cast<uint64_t>(i) << 1) + j + t.bits[i & 7];
}
__attribute__((noinline)) uint64_t free_mask(const Table& t, uint64_t a, uint64_t b, uint32_t kind)
{
	uint64_t m = kind == 1 ? 0xf3cf : kind == 2 ? 0xff0000 : 0;
	m &= static_cast<uint32_t>(~a);
	m &= static_cast<uint32_t>(~t.bits[1]);
	m &= static_cast<uint32_t>(-b);
	return m;
}
__attribute__((noinline)) uint64_t scaled(uint64_t x, uint64_t y)
{
	const uint32_t a = static_cast<uint32_t>(x << 3);
	const uint32_t b = static_cast<uint32_t>(y * 0x9e3779b97f4a7c15ull);
	const uint32_t c = static_cast<uint32_t>(x ^ y) | static_cast<uint32_t>(x & 0xffffffff00000000ull);
	return static_cast<uint64_t>(a) + b + c;
}
__attribute__((noinline)) uint64_t low_halves(const Table& t, unsigned k)
{
	uint64_t s = 0;
	for (unsigned i = 0; i < 8; ++i)
		s += static_cast<uint32_t>(t.sizes[(i + k) & 7]);
	return s;
}
__attribute__((noinline)) uint64_t moved(const uint64_t* v, uint32_t a, uint32_t b, signed char c, uint32_t d)
{
	uint64_t s = 0;
	for (uint32_t i = 0; i < 3; ++i)
		s += v[(a + i) & 7] * b + static_cast<uint64_t>(c) + d;
	return s ^ (static_cast<uint64_t>(a) << 32) ^ b;
}
int main()
{
	Table t = {};
	for (unsigned i = 0; i < 8; ++i)
	{
		t.sizes[i] = 0x100000000ull * (i + 1) + 0xfffffff0u + i;
		t.bits[i] = 0x01010101u * (i + 3);
	}
	const uint64_t ns[] = { 0, 1, 0x100000000ull, 0xffffffffffffffffull, 12345 };
	for (uint64_t n : ns)
		std::printf("last_index %llu\n", static_cast<unsigned long long>(last_index(t, n, 0xfffffffffull)));
	for (uint32_t kind = 0; kind < 3; ++kind)
		std::printf("free_mask %llx\n", static_cast<unsigned long long>(free_mask(t, 0x123456789abcull, 0x10, kind)));
	std::printf("scaled %llx %llx\n", static_cast<unsigned long long>(scaled(0x1234567890ull, 77)),
		static_cast<unsigned long long>(scaled(0xffffffffffffffffull, 0x8000000000000000ull)));
	std::printf("low_halves %llx %llx\n", static_cast<unsigned long long>(low_halves(t, 0)),
		static_cast<unsigned long long>(low_halves(t, 5)));
	const uint64_t v[8] = { 1, 2, 3, 4, 5, 6, 7, 0xffffffffffull };
	std::printf("moved %llx %llx\n", static_cast<unsigned long long>(moved(v, 6, 0xffffffffu, -3, 7)),
		static_cast<unsigned long long>(moved(v, 0xfffffffeu, 3, 100, 0x80000000u)));
	return 0;
}
