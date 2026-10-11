// Extension sinking (O1+) moves an in-place extension down to the block
// dominating its wide reads, but the MIR CFG has no edges from calls to
// their landing pads: a read a landing pad reaches (the join after a
// catch) keeps the extension in place.
#include <cstdio>

__attribute__((noinline)) long check(long x)
{
	if (x % 2 == 0)
		throw 1;
	return x;
}

__attribute__((noinline)) unsigned long f(unsigned long p0, long p1)
{
	unsigned short v = static_cast<unsigned short>(p0 >> 5);
	long t = 0;
	try
	{
		check(p1);   // throws for even p1
		t = (p1 & 4) ? 1 : 2;
		t += check(static_cast<long>(v) * 255 + t + 1);
	}
	catch (...)
	{
	}
	return static_cast<unsigned long>(v) + static_cast<unsigned long>(t);
}

int main(int argc, char**)
{
	for (long k = 0; k < 4; ++k)
		std::printf("%ld %lu\n", k, f(static_cast<unsigned long>(k + argc) * 0x9e3779b97f4a7c15ul, k));
	return 0;
}
