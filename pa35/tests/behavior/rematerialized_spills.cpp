// Spilled frame addresses and constants recomputed at their uses (O1+:
// a register whose one definition is a frame address or a constant has no
// spill slot): local objects whose addresses stay live across calls under
// register pressure, copied whole and through fields.
#include <cstdio>
#include <cstdint>
#include <cstring>
struct Pair { uint64_t a, b; };
struct Big { Pair p[4]; uint32_t tag; };
__attribute__((noinline)) void touch(Pair* p, uint64_t k) { p->a += k; p->b ^= k * 3; }
__attribute__((noinline)) uint64_t mix(uint64_t a, uint64_t b, uint64_t c, uint64_t d) { return a * 3 + b * 5 + c * 7 + d * 11; }
__attribute__((noinline)) uint64_t work(const Pair* in, int n)
{
	Pair x = in[0], y = in[1], z = in[2];
	Big big;
	std::memset(&big, 0, sizeof big);
	uint64_t s0 = 1, s1 = 2, s2 = 3, s3 = 4, s4 = 5, s5 = 6, s6 = 7, s7 = 8;
	for (int i = 0; i < n; ++i)
	{
		touch(&x, static_cast<uint64_t>(i));
		touch(&y, s0);
		touch(&big.p[i & 3], s1 + 0x1234567890ull);
		big.p[(i + 1) & 3] = z;
		s0 += x.a ^ s7;
		s1 += y.b + s6;
		s2 ^= big.p[i & 3].a + s5;
		s3 += mix(s0, s1, s2, x.b);
		s4 ^= s3 + 0x7fffffffull;
		s5 += s4 * 3;
		s6 ^= s5 + y.a;
		s7 += s6 + big.tag++;
		z.a += s7;
	}
	return s0 + s1 + s2 + s3 + s4 + s5 + s6 + s7 + x.a + y.b + z.a + big.p[0].b + big.p[3].a + big.tag;
}
int main()
{
	Pair in[3] = { { 1, 2 }, { 3, 4 }, { 5, 6 } };
	std::printf("%llu\n", static_cast<unsigned long long>(work(in, 37)));
	return 0;
}
