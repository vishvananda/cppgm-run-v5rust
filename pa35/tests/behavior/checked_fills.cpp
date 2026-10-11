// Fill loops of 2-, 4- and 8-byte values the compiler does not know (host
// objects: a memset when the value's bytes are alike, checked before the loop,
// which runs otherwise), loops counted by a down-counter (`while (n--)`)
// whose pointer is used after them, and vector::assign (alike and unlike
// values, growing and shrinking).
#include <cstdio>
#include <cstdint>
#include <vector>
__attribute__((noinline)) void fill_count(unsigned* p, unsigned long n, const unsigned& v)
{
	for (; n--; ++p)
		*p = v;
}
__attribute__((noinline)) unsigned* fill_count_end(unsigned* p, unsigned long n, const unsigned& v)
{
	for (; n--; ++p)
		*p = v;
	return p;
}
__attribute__((noinline)) void fill_range(unsigned short* p, unsigned short* e, unsigned short v)
{
	for (; p != e; ++p)
		*p = v;
}
__attribute__((noinline)) void fill_u64(uint64_t* p, int n, uint64_t v)
{
	for (int i = 0; i < n; ++i)
		p[i] = v;
}
int main()
{
	unsigned a[40];
	unsigned short b[40];
	uint64_t c[20];
	unsigned long sum = 0;
	for (unsigned k = 0; k < 5; ++k)
	{
		const unsigned vals[5] = { 0u, 0xffffffffu, 0x01020304u, 0x7f7f7f7fu, 5u };
		for (auto& x : a) x = 99;
		fill_count(a + 1, 30 - k, vals[k]);
		unsigned* e = fill_count_end(a + 5, k, vals[(k + 1) % 5]);
		for (unsigned x : a) sum = sum * 31 + x;
		sum += static_cast<unsigned long>(e - a);
		for (auto& x : b) x = 7;
		fill_range(b + 2, b + 2 + 3 * k, static_cast<unsigned short>(vals[k] & 0xffff));
		for (unsigned short x : b) sum = sum * 31 + x;
		for (auto& x : c) x = 3;
		fill_u64(c, static_cast<int>(4 * k), k == 2 ? 0x0101010101010101ull : static_cast<uint64_t>(vals[k]) * 0x100000001ull);
		for (uint64_t x : c) sum = sum * 31 + x;
	}
	std::vector<unsigned> v(10, 3);
	for (unsigned n : { 5u, 20u, 7u, 40u, 0u, 3u })
	{
		v.assign(n, n == 20 ? 0xffffffffu : n);
		for (unsigned x : v) sum = sum * 31 + x;
		sum += v.size();
	}
	std::printf("%lu\n", sum);
	return 0;
}
