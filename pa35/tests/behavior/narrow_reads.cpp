// Extensions of registers only read narrowly (host O1+ over virtual
// registers): a bool a call returns and a branch tests, bytes from compares
// joined and compared as bytes, a u32 stored as 32 bits; beside them the
// same kinds of values read whole (an index, a 64-bit sum, a copy returned)
// keep their extensions.
#include <cstdint>
#include <cstdio>

__attribute__((noinline)) bool odd(int x) { return (x & 1) != 0; }
__attribute__((noinline)) unsigned char klass(unsigned x) { return static_cast<unsigned char>(x % 5); }

struct Table { unsigned char cls[64]; uint32_t out[64]; };

__attribute__((noinline)) bool same(const Table& t, unsigned u, unsigned v)
{
	const unsigned char cu = u < 8 ? (u >= 4) : t.cls[u];
	const unsigned char cv = v < 8 ? (v >= 4) : t.cls[v];
	return cu == cv;
}

__attribute__((noinline)) long weigh(const Table& t, unsigned u)
{
	// Read whole: the byte indexes and joins a 64-bit sum.
	const unsigned char c = u < 8 ? (u >= 4) : t.cls[u];
	return static_cast<long>(c) * 1000 + t.out[c];
}

__attribute__((noinline)) void store(Table& t, unsigned k, unsigned a, unsigned b)
{
	t.out[k & 63] = a * 3 + b;
}

int main()
{
	Table t;
	for (unsigned i = 0; i < 64; ++i)
	{
		t.cls[i] = klass(i * 7);
		t.out[i] = 0;
	}
	long total = 0;
	unsigned matches = 0;
	for (int i = 0; i < 200; ++i)
	{
		if (odd(i))
			total += i;
		const bool flag = odd(i * 3 + 1);
		total += flag;
		for (unsigned v = 0; v < 64; v += 9)
			if (same(t, static_cast<unsigned>(i) % 64, v))
				++matches;
		store(t, static_cast<unsigned>(i), static_cast<unsigned>(i) * 2654435761u, 7);
		total += weigh(t, static_cast<unsigned>(i) % 64);
	}
	unsigned long h = 0;
	for (unsigned i = 0; i < 64; ++i)
		h = h * 31 + t.out[i];
	std::printf("%ld %u %lu\n", total, matches, h);
	return 0;
}
