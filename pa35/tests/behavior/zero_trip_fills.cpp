// Fill loops (host O1+) that read their value through a pointer read it in
// the preheader only where a pass of the loop would: a loop of no passes
// with a null source never reads it. A 16-byte literal fills by bytes only
// when all sixteen are alike.
#include <cstdint>
#include <cstdio>

volatile unsigned none = 0, two = 2;

struct B2
{
	unsigned char a, b;
};

__attribute__((noinline)) void bytes(char* p, const char* s, unsigned n)
{
	while (n--)
		*p++ = *s;
}

__attribute__((noinline)) void words(uint32_t* p, const uint32_t* s, unsigned n)
{
	while (n--)
		*p++ = *s;
}

__attribute__((noinline)) void objects(B2* p, const B2* s, unsigned n)
{
	while (n--)
		*p++ = *s;
}

__attribute__((noinline)) void wide(unsigned __int128* p, unsigned n)
{
	while (n--)
		*p++ = static_cast<unsigned __int128>(1) << 64;   // bytes 00 x8, 01, 00 x7
}

int main()
{
	char c[4] = "abc";
	uint32_t w[2] = { 1, 2 };
	B2 b[2] = {};
	bytes(c, nullptr, none);
	words(w, nullptr, none);
	objects(b, nullptr, none);
	std::printf("%s %u %u %u\n", c, w[0], w[1], b[1].a);
	const char x = 'z';
	const uint32_t y = 0x01010101u;
	const B2 z = { 7, 7 };
	bytes(c, &x, two);
	words(w, &y, two);
	objects(b, &z, two);
	std::printf("%s %x %x %u %u\n", c, w[0], w[1], b[1].a, b[1].b);
	unsigned __int128 a[2] = {};
	wide(a, two);
	std::printf("%llx %llx\n", static_cast<unsigned long long>(a[1] >> 64), static_cast<unsigned long long>(a[1]));
	return 0;
}
