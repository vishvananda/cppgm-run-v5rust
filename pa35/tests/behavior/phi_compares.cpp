// A branch on a compare of a phi (O2+: each predecessor compares its own
// incoming value on its edge): UTF-8 widths chosen on three paths and
// tested, both operand orders, a phi of a value and literals, a switch
// predecessor reaching the compare from several cases.
#include <cstdio>
#include <cstdint>
__attribute__((noinline)) int width_matches(uint32_t c, uint32_t got)
{
	uint32_t want;
	if (c < 0x80)
		want = 1;
	else if (c < 0x800)
		want = 2;
	else if (c < 0x10000)
		want = 3;
	else
		return got == 4 ? 7 : 8;
	if (got == want)
		return 10;
	return 20;
}
__attribute__((noinline)) int reversed(int k, int x)
{
	int lim;
	if (k > 0)
		lim = x;
	else if (k < -5)
		lim = 3;
	else
		lim = -1;
	if (lim < x - 1)
		return 1;
	return 2;
}
__attribute__((noinline)) int from_switch(int k, int y)
{
	int v;
	switch (k)
	{
	case 0:
	case 1:
		v = 4;
		break;
	case 2:
		v = y;
		break;
	default:
		v = 9;
		break;
	}
	if (y == v)
		return k + 100;
	return k;
}
int main()
{
	const uint32_t cs[] = { 'a', 0x7ff, 0x800, 0xffff, 0x10000, 0x10ffff };
	unsigned s = 0;
	for (uint32_t c : cs)
		for (uint32_t g = 0; g < 6; ++g)
			s = s * 3 + static_cast<unsigned>(width_matches(c, g));
	for (int k = -8; k < 4; ++k)
		for (int x = -3; x < 5; ++x)
			s = s * 5 + static_cast<unsigned>(reversed(k, x));
	for (int k = 0; k < 5; ++k)
		for (int y = 2; y < 10; ++y)
			s = s * 7 + static_cast<unsigned>(from_switch(k, y));
	std::printf("%u\n", s);
	return 0;
}
