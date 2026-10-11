// 128-bit integers as pairs of words (host O2+ wide integer splitting):
// shifts by every count (constant and variable, left, logical and
// arithmetic right), carries and borrows across the halves, signed and
// unsigned compares deciding in either half, negation and complements,
// extensions and truncations, values in memory and in loops, and values
// crossing calls, multiplications and divisions (kept whole).
#include <cstdint>
#include <cstdio>

typedef unsigned __int128 u128;
typedef __int128 s128;

static unsigned long fold(u128 v) { return static_cast<unsigned long>(v) * 31 + static_cast<unsigned long>(v >> 64); }

__attribute__((noinline)) u128 make(uint64_t hi, uint64_t lo) { return (static_cast<u128>(hi) << 64) | lo; }

__attribute__((noinline)) unsigned long shifts(u128 v, unsigned n)
{
	unsigned long h = 0;
	h = h * 7 + fold(v << n);
	h = h * 7 + fold(v >> n);
	h = h * 7 + fold(static_cast<u128>(static_cast<s128>(v) >> n));
	h = h * 7 + fold((static_cast<u128>(1) << n) - 1);
	return h;
}

__attribute__((noinline)) unsigned long constant_shifts(u128 v)
{
	unsigned long h = 0;
	h = h * 7 + fold(v << 1) + fold(v << 63) + fold(v << 64) + fold(v << 65) + fold(v << 127);
	h = h * 7 + fold(v >> 1) + fold(v >> 63) + fold(v >> 64) + fold(v >> 65) + fold(v >> 127);
	const s128 s = static_cast<s128>(v);
	h = h * 7 + fold(static_cast<u128>(s >> 1)) + fold(static_cast<u128>(s >> 64)) + fold(static_cast<u128>(s >> 100));
	return h;
}

__attribute__((noinline)) unsigned long arithmetic(u128 a, u128 b)
{
	unsigned long h = 0;
	h = h * 7 + fold(a + b) + fold(a - b) + fold(b - a);
	h = h * 7 + fold(-a) + fold(~b) + fold(a & b) + fold(a | b) + fold(a ^ b);
	h = h * 7 + (a == b) + 2 * (a != b) + 4 * (a < b) + 8 * (a <= b) + 16 * (a > b) + 32 * (a >= b);
	const s128 x = static_cast<s128>(a), y = static_cast<s128>(b);
	h = h * 7 + (x < y) + 2 * (x <= y) + 4 * (x > y) + 8 * (x >= y);
	h = h * 7 + fold(a * b) + (b != 0 ? fold(a / b) + fold(a % b) : 0);
	return h;
}

__attribute__((noinline)) unsigned long conversions(int64_t s, uint64_t u, int32_t i, unsigned char c)
{
	const s128 a = s;
	const u128 b = u;
	const s128 d = i;
	const u128 e = c;
	return fold(static_cast<u128>(a)) * 3 + fold(b) * 5 + fold(static_cast<u128>(d)) * 7 + fold(e) +
		static_cast<unsigned long>(static_cast<uint32_t>(b + a)) + static_cast<unsigned char>(a >> 70);
}

struct Holder { u128 values[8]; s128 total; };

__attribute__((noinline)) void fill(Holder& h, u128 seed)
{
	h.total = 0;
	for (int k = 0; k < 8; ++k)
	{
		h.values[k] = seed;
		h.total += static_cast<s128>(seed >> 8) - k;
		seed = seed * 6364136223846793005ull + 1442695040888963407ull;
		seed ^= seed >> 67;
	}
}

__attribute__((noinline)) u128 sum(const Holder& h)
{
	u128 s = 0;
	for (int k = 0; k < 8; ++k)
		if (h.values[k] > s)
			s += h.values[k] >> (k * 13);
		else
			s -= h.values[k];
	return s;
}

int main()
{
	const u128 values[] = { 0, 1, ~static_cast<u128>(0), make(0, ~0ull), make(1, 0), make(0x8000000000000000ull, 0),
		make(0x7fffffffffffffffull, ~0ull), make(0x123456789abcdefull, 0xfedcba9876543210ull) };
	const unsigned count = sizeof values / sizeof values[0];
	unsigned long h = 0;
	for (unsigned i = 0; i < count; ++i)
	{
		for (unsigned n = 0; n < 128; ++n)
			h = h * 3 + shifts(values[i], n);
		h = h * 3 + constant_shifts(values[i]);
		for (unsigned j = 0; j < count; ++j)
			h = h * 3 + arithmetic(values[i], values[j]);
	}
	h = h * 3 + conversions(-5, 7, -9, 200) + conversions(INT64_MIN, ~0ull, INT32_MIN, 255);
	Holder holder;
	fill(holder, values[7]);
	h = h * 3 + fold(sum(holder)) + fold(static_cast<u128>(holder.total));
	std::printf("%lu\n", h);
	return 0;
}
