// Loop counters stepped by one below a bound of their own type (O2+:
// IF_NO_WRAP, their 64-bit sums kept unrenormalized): int counters from
// negative and positive starts, used as indices and values; u8 and i16
// counters near their types' limits; a latch-tested counter from a
// literal; a counter that leaves early; one compared against INT_MAX; and
// one whose bound is a wider type (not marked).
#include <climits>
#include <cstdint>
#include <cstdio>
__attribute__((noinline)) unsigned orbits(const unsigned* bits, int a, int b)
{
	unsigned m = 0;
	for (int k = a; k < b; ++k)
		m |= bits[k + 8];
	return m;
}
__attribute__((noinline)) long signed_values(int a, int b)
{
	long s = 0;
	for (int k = a; k < b; ++k)
		s = s * 3 + k;
	return s;
}
__attribute__((noinline)) unsigned bytes(const unsigned char* p, unsigned char limit)
{
	unsigned s = 0;
	for (unsigned char r = 0; r < limit; ++r)
		s = s * 31 + p[r] + r;
	return s;
}
__attribute__((noinline)) int shorts(short from, short to)
{
	int s = 0;
	for (short k = from; k < to; ++k)
		s += k;
	return s;
}
__attribute__((noinline)) int latch(const int* v, int n)
{
	int s = 0;
	int k = 0;
	do
	{
		s += v[k];
		++k;
	} while (k < n);
	return s;
}
__attribute__((noinline)) int early(const int* v, int n, int stop)
{
	for (int k = 0; k < n; ++k)
		if (v[k] == stop)
			return k;
	return -1;
}
__attribute__((noinline)) long near_max(int from)
{
	long s = 0;
	for (int k = from; k < INT_MAX; ++k)
		s += k;
	return s;
}
__attribute__((noinline)) long wide_bound(int from, long bound)
{
	long s = 0;
	for (int k = from; k < bound; ++k)
		s += k;
	return s;
}
int main()
{
	unsigned bits[64];
	unsigned char p[256];
	int v[32];
	for (int i = 0; i < 64; ++i)
		bits[i] = 1u << (i % 32);
	for (int i = 0; i < 256; ++i)
		p[i] = static_cast<unsigned char>(i * 37);
	for (int i = 0; i < 32; ++i)
		v[i] = i * i - 50;
	std::printf("%x %x %ld %ld\n", orbits(bits, -8, 20), orbits(bits, 5, 5), signed_values(-20, 7), signed_values(3, -3));
	std::printf("%u %u %u\n", bytes(p, 0), bytes(p, 16), bytes(p, 255));
	std::printf("%d %d %d\n", shorts(-32768, -32700), shorts(32700, 32767), shorts(5, 1));
	std::printf("%d %d %d %d\n", latch(v, 1), latch(v, 32), early(v, 32, 50), early(v, 32, 7));
	std::printf("%ld %ld %ld\n", near_max(INT_MAX - 5), wide_bound(-3, 4), wide_bound(2147483640, 2147483647L));
	return 0;
}
