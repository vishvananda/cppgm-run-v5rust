// Tests a jumping block decides (O1+ MIR threading): switch cases that
// choose a table and meet at its null test; rotated loops whose entry
// tests a constant start, at every width and signedness, including starts
// that wrap or sit at a sign boundary; constants chosen on two paths and
// compared after the merge.
#include <cstdio>
#include <cstdint>
static const char* const kLetters[] = { "a", "bb", "ccc", nullptr };
static const char* const kDigits[] = { "0", "11", nullptr };
static const char* const kNone[] = { nullptr };
static const char* const* Candidates(int c)
{
	switch (c)
	{
	case 'a':
	case 'b':
		return kLetters;
	case '0':
	case '1':
		return kDigits;
	case 'z':
		return kNone;
	default:
		return nullptr;
	}
}
__attribute__((noinline)) int count(int c)
{
	const char* const* p = Candidates(c);
	if (p == nullptr)
		return -1;
	int n = 0;
	for (; *p != nullptr; ++p)
		n += 1 + (**p & 3);
	return n;
}
__attribute__((noinline)) unsigned mask(const int* starts, const int* ends, int pos)
{
	unsigned m = 0;
	for (int r = 0; r < 3; ++r)
		if (starts[r] < pos || ends[r] <= pos)
			m |= 1u << r;
	return m;
}
__attribute__((noinline)) int signed_bytes(int bias)
{
	int s = 0;
	for (signed char i = -3; i < 2; ++i)
		s = s * 3 + i + bias;
	return s;
}
__attribute__((noinline)) unsigned wrapping(unsigned step)
{
	unsigned n = 0;
	for (unsigned i = 0xfffffff0u; i != 0; i += step)
		++n;
	return n;
}
__attribute__((noinline)) long long wide_down(long long k)
{
	long long s = 0;
	for (long long i = -1; i >= -5; --i)
		s += i * k;
	return s;
}
__attribute__((noinline)) int shorts(int k)
{
	int n = 0;
	for (unsigned short s = 65530; s > 10; s = static_cast<unsigned short>(s + 3))
		n += k;
	return n;
}
__attribute__((noinline)) int edges(int c, int x)
{
	int k;
	long long w;
	if (c > 0)
	{
		k = -7;
		w = 0x100000000LL;
	}
	else
	{
		k = 7;
		w = -1;
	}
	int r = 0;
	if (k < 0)
		r += x;
	if (static_cast<unsigned>(k) > 100u)
		r += 2 * x;
	if (static_cast<uint32_t>(w) == 0)
		r += 4 * x;
	if (w < 0)
		r += 8 * x;
	return r;
}
int main()
{
	const char probes[] = "ab01z?";
	for (const char* p = probes; *p; ++p)
		std::printf("count %c %d\n", *p, count(*p));
	const int starts[] = { 1, 5, 9 }, ends[] = { 4, 6, 3 };
	for (int pos = 0; pos < 11; pos += 2)
		std::printf("mask %d %u\n", pos, mask(starts, ends, pos));
	std::printf("signed_bytes %d %d\n", signed_bytes(0), signed_bytes(5));
	std::printf("wrapping %u %u\n", wrapping(1), wrapping(4));
	std::printf("wide_down %lld\n", wide_down(3));
	std::printf("shorts %d\n", shorts(2));
	std::printf("edges %d %d\n", edges(1, 1), edges(-1, 1));
	return 0;
}
