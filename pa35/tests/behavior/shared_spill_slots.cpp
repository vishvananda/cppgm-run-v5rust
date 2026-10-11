// Spilled registers sharing frame slots (O1+ over virtual registers: ones
// that are never live together take one slot): integer and floating values
// live across calls in successive groups, more than the registers hold, so
// that later groups reuse earlier groups' slots and no integer value shares
// one with a floating value live at the same time.
#include <cstdio>

__attribute__((noinline)) long touch(long x) { return x * 3 + 1; }
__attribute__((noinline)) double dtouch(double x) { return x * 0.5 + 1.0; }

__attribute__((noinline)) double groups(long seed)
{
	long a0 = touch(seed), a1 = touch(a0), a2 = touch(a1), a3 = touch(a2), a4 = touch(a3), a5 = touch(a4),
		 a6 = touch(a5), a7 = touch(a6), a8 = touch(a7);
	double d0 = dtouch(static_cast<double>(a0)), d1 = dtouch(d0), d2 = dtouch(d1), d3 = dtouch(d2);
	const long first = a0 + a1 * 2 + a2 * 3 + a3 * 4 + a4 * 5 + a5 * 6 + a6 * 7 + a7 * 8 + a8 * 9;
	const double dfirst = d0 + d1 * 2 + d2 * 3 + d3 * 4;
	long b0 = touch(first), b1 = touch(b0), b2 = touch(b1), b3 = touch(b2), b4 = touch(b3), b5 = touch(b4),
		 b6 = touch(b5), b7 = touch(b6), b8 = touch(b7);
	double e0 = dtouch(dfirst), e1 = dtouch(e0), e2 = dtouch(e1), e3 = dtouch(e2);
	const long second = b0 - b1 + b2 - b3 + b4 - b5 + b6 - b7 + b8;
	const double dsecond = e0 - e1 + e2 - e3;
	long c0 = touch(second), c1 = touch(c0), c2 = touch(c1), c3 = touch(c2), c4 = touch(c3), c5 = touch(c4),
		 c6 = touch(c5), c7 = touch(c6);
	double f0 = dtouch(dsecond), f1 = dtouch(f0), f2 = dtouch(f1);
	return static_cast<double>(c0 ^ c1 ^ c2 ^ c3 ^ c4 ^ c5 ^ c6 ^ c7) + f0 + f1 * 3 + f2 * 5 +
		static_cast<double>(first % 1000) + dfirst;
}

int main()
{
	double t = 0;
	for (long s = 1; s < 40; ++s)
		t += groups(s) * 1e-12;
	std::printf("%.6e\n", t);
	return 0;
}
