// Shrink-wrapping (host O2+) sinks an entry copy of a caller-saved
// register into a preserved one to the prologue block S and renames the
// preserved register before S: its writes there may land in the source
// register only when nothing after S still reads that register by its own
// name (an argument, or a value the entry computed), and no other copy or
// store moved to S reads it; otherwise the value waits in a relay.
#include <cstdint>
#include <cstdio>

typedef unsigned long U;

namespace computed
{
// `mov rbx, rax` (rax holds d): the loop's writes of rbx before S must not
// clobber d, which the framed part reads.
__attribute__((noinline)) U ext(U x, U y) { return x + 2 * y; }
__attribute__((noinline)) U f(U a, U b, U c, U d)
{
	U v3 = d;
	for (U j = 0; j < (b & 7); ++j)
		v3 += 15 ^ j;
	U v4 = a << (v3 & 31);
	if (((v3 * d) % 3) == 0)
		return v4 + c;
	U v0 = a;
	for (U j = 0; j < (v3 & 3); ++j)
		v0 += ext(j + v4, a);
	return v0 + v3 * 7 + v4 * 11;
}
}

namespace argument
{
// `mov rbp, rdi; xor rbp, 58`: the call after S still passes a itself.
__attribute__((noinline)) U thrower(U x, U y)
{
	if ((x ^ y) % 13 == 5)
		throw static_cast<int>((x + y) & 0xffff);
	return x + 2 * y;
}
__attribute__((noinline)) U ext10(U a, U b, U c, U d, U e, U f, U g, U h, U i, U j)
{
	return a + 2 * b + 3 * c + 4 * d + 5 * e + 6 * f + 7 * g + 8 * h + 9 * i + 10 * j;
}
__attribute__((noinline)) U f(U a, U b, U c, U d)
{
	U v0 = a, v1 = b, v2 = c, v3 = d, v4 = a ^ 58, v5 = b + 24;
	if (((v5 + v2) % 2) == 0)
		return b * b * 8 + v3;
	if ((((38 << (v0 & 31)) >> 3) & 1) != 0)
	{
		U arr[6] = { 0, 0, 0, 0, 0, d };
		arr[d % 6] = 89;
		v5 += arr[60 % 6] + arr[5];
	}
	v0 += ext10(v0, v1, v2, v3, v4, v5, a, b, v5, 55);
	{
		unsigned __int128 w = static_cast<unsigned __int128>(d) * 35;
		v2 = static_cast<U>(w / ((static_cast<unsigned __int128>(v5) | 1) + 1000000007ull));
	}
	v0 += thrower(v0 / (c | 1), static_cast<unsigned char>(v1 + v1));
	return v0 + v1 * 3 + v2 * 5 + v3 * 7 + v4 * 11 + v5 * 13;
}
}

namespace shared
{
// Two copies of one argument (rdi into r12 and rbx), each written before
// S: neither may take the argument register's name.
long table[16];
__attribute__((noinline)) void g() { table[3] += 1; }
__attribute__((noinline)) long f(unsigned long p, long q)
{
	unsigned long a = p & 15;
	unsigned long b = p ^ 7;
	if (q == 0)
		return static_cast<long>(a * 3 + b);
	for (long i = 0; i < q; ++i)
	{
		g();
		table[a] += static_cast<long>(b);
	}
	return static_cast<long>(a + b) + table[a];
}
}

namespace fields
{
volatile uint64_t vx = 0x9e3779b97f4a7c15ull;
struct S4
{
	int64_t f0;
	int16_t f1;
	int8_t f2, f3;
	uint8_t f4;
	int8_t f5;
	uint8_t f6, f7;
};
__attribute__((noinline)) uint64_t f(uint64_t x, int k)
{
	uint64_t h = 0;
	S4 a = S4();
	S4 b;
	b = a;
	b.f3 = static_cast<int8_t>(x >> 13);
	a.f1 = static_cast<int16_t>(-k);
	b.f2 = static_cast<int8_t>(-k);
	if (k & 1)
		a.f4 = static_cast<uint8_t>(a.f4 + 19);
	a.f5 = static_cast<int8_t>(k);
	a.f0 = static_cast<int64_t>(x);
	if (k & 2)
		a.f4 = static_cast<uint8_t>(a.f4 + 99);
	h = h * 131 + static_cast<uint64_t>(a.f0) + static_cast<uint64_t>(b.f0) * 3;
	h = h * 131 + static_cast<uint64_t>(static_cast<int64_t>(a.f1)) + static_cast<uint64_t>(b.f1) * 3;
	h = h * 131 + static_cast<uint64_t>(static_cast<int64_t>(a.f2)) + static_cast<uint64_t>(b.f2) * 3;
	h = h * 131 + static_cast<uint64_t>(static_cast<int64_t>(a.f3)) + static_cast<uint64_t>(b.f3) * 3;
	h = h * 131 + static_cast<uint64_t>(static_cast<int64_t>(a.f5)) + static_cast<uint64_t>(b.f5) * 3;
	return h;
}
}

namespace narrow
{
int8_t g8[16];
uint8_t gu8[16];
int16_t g16[16];
uint32_t gu32[16];
uint64_t gu64[16];
uint64_t acc;
__attribute__((noinline)) int32_t f0(uint32_t p0, uint16_t p1) { return p0 + p1; }
__attribute__((noinline)) int16_t f1(uint64_t p0, uint32_t p1)
{
	acc = acc * 1000003u + static_cast<uint64_t>(static_cast<int16_t>(~static_cast<uint32_t>(static_cast<int16_t>(
		p1 % (p1 | 1) - static_cast<uint32_t>(static_cast<int8_t>(static_cast<uint32_t>(g8[gu32[12] & 15]) << 5))))));
	if (static_cast<uint32_t>(static_cast<int16_t>(static_cast<uint32_t>(gu8[0]) ^ gu8[static_cast<uint32_t>(p0) & 15]) -
			static_cast<uint32_t>(~p0)) == 4294967259u)
		acc = acc * 31 + 4;
	{
		uint64_t* q = &gu64[(p1 * static_cast<uint32_t>(g8[0])) & 15];
		*q |= 0;
	}
	{
		int16_t* q = &g16[(0 - static_cast<uint32_t>(p0)) & 15];
		*q ^= 61;
	}
	f0(0u - gu8[12], 65535);
	return g16[11];
}
}

int main(int argc, char**)
{
	std::printf("%lu\n", computed::f(argc, 2, 5, 3));
	std::printf("%lu\n", argument::f(argc, 0, 1, 13));
	unsigned long h = 0;
	for (unsigned long p = 0; p < 40; p += 3)
		h = h * 31 + shared::f(p, p & 3) + shared::f(p, 0);
	std::printf("%lu\n", h);
	for (int k = 0; k < 8; ++k)
		std::printf("%d %llu\n", k, static_cast<unsigned long long>(fields::f(fields::vx + k * 77777, 7 + k * 13)));
	for (int r = 0; r < 8; ++r)
		std::printf("%d ", narrow::f1(r, 300));
	std::printf("\n");
	return 0;
}
