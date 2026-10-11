// Argument extensions moved to where the values are read whole (O1+ over
// virtual registers): u32 and int parameters that early exits read as 32
// bits and later code uses as indexes, 64-bit products and hash keys;
// callers pass truncations of 64-bit values, whose upper register bits the
// callee must not see.
#include <cstdint>
#include <cstdio>
#include <vector>

struct Table
{
	std::vector<unsigned char> cls;
	std::vector<uint64_t> keys;
	uint64_t count = 0;
	__attribute__((noinline)) void add(unsigned u, unsigned v)
	{
		if (u == v || (u < 8 && v < 8))
			return;
		const unsigned cu = u < 8 ? 1 : cls[u - 8];
		const unsigned cv = v < 8 ? 1 : cls[v - 8];
		if (cu != cv)
			return;
		keys.push_back((static_cast<uint64_t>(u) << 32 | v) * 3);
		count += static_cast<uint64_t>(u) * 24 + v;
	}
};

__attribute__((noinline)) long signed_use(int k, const long* base)
{
	if (k == 0)
		return -1;
	return base[k + 2] + static_cast<long>(k) * 1000;
}

int main()
{
	Table t;
	t.cls.resize(300);
	for (unsigned i = 0; i < 300; ++i)
		t.cls[i] = static_cast<unsigned char>(i % 3);
	uint64_t big = 0x123456789ull;
	for (unsigned i = 0; i < 200; ++i)
	{
		big = big * 6364136223846793005ull + 1442695040888963407ull;
		t.add(static_cast<unsigned>(big >> 33) % 300, static_cast<unsigned>(big % 300 + (big & ~0xffffffffull)));
	}
	long values[16];
	for (int i = 0; i < 16; ++i)
		values[i] = i * i;
	long s = 0;
	for (long k = -2; k < 12; ++k)
		s += signed_use(static_cast<int>(k + (static_cast<long>(1) << 40)), values);
	uint64_t h = t.count;
	for (uint64_t k : t.keys)
		h = h * 31 + k;
	std::printf("%llu %ld\n", static_cast<unsigned long long>(h), s);
	return 0;
}
