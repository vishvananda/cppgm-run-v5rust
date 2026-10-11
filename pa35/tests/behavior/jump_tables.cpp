// Switches through jump tables (host objects: at least six cases filling a
// quarter of their span): distinct targets, gaps that take the default,
// narrow unsigned and negative selectors.
#include <cstdio>
#include <cstdint>
__attribute__((noinline)) int dispatch(int op, int x)
{
	switch (op)
	{
	case 0: return x + 1;
	case 1: return x * 3;
	case 2: return x - 7;
	case 3: return x ^ 0x55;
	case 5: return x * 4;
	case 6: return -x;
	case 7: return x / 3;
	case 9: return x % 11;
	case 10: return x & 0xf0;
	default: return 42;
	}
}
__attribute__((noinline)) unsigned char cls(unsigned char c)
{
	switch (c)
	{
	case 'a': return 1; case 'e': return 2; case 'i': return 3; case 'o': return 4; case 'u': return 5;
	case 'y': return 6; case 'b': return 7; case 'c': return 8; case 'd': return 9; case 'z': return 10;
	default: return 0;
	}
}
__attribute__((noinline)) long neg(long v)
{
	switch (v)
	{
	case -5: return 100; case -4: return 200; case -3: return 300; case -1: return 400;
	case 0: return 500; case 1: return 600; case 3: return 700;
	default: return -1;
	}
}
int main()
{
	unsigned h = 0;
	for (int op = -3; op < 14; ++op)
		for (int x = -5; x < 6; ++x)
			h = h * 31 + static_cast<unsigned>(dispatch(op, x));
	for (int c = 0; c < 256; ++c)
		h = h * 7 + cls(static_cast<unsigned char>(c));
	for (long v = -8; v < 8; ++v)
		h = h * 13 + static_cast<unsigned>(neg(v));
	std::printf("%u\n", h);
	return 0;
}
