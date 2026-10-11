// Chains of equality tests on one value (host O2+: one switch, then bit
// tests): `||` and `&&` forms, byte, int and long values, chains that end
// in a switch already there, a target whose phi takes different values per
// test (kept as compares), repeated literals and negative ones.
#include <cstdint>
#include <cstdio>

__attribute__((noinline)) bool space(int c) { return c == ' ' || c == '\t' || c == '\v' || c == '\f' || c == '\r'; }

__attribute__((noinline)) bool not_digitish(unsigned char c)
{
	return c != '0' && c != '1' && c != '7' && c != '9';
}

__attribute__((noinline)) int classify(long k)
{
	if (k == -3 || k == 5 || k == 40 || k == 41 || k == 63)
		return 1;
	if (k == 100 || k == 101 || k == 100)
		return 2;
	return 0;
}

__attribute__((noinline)) int weights(int c)
{
	int w = 0;
	if (c == 1)
		w = 10;
	else if (c == 2)
		w = 20;
	else if (c == 3)
		w = 30;
	else if (c == 4)
		w = 40;
	return w + c;
}

__attribute__((noinline)) int dispatch(int c)
{
	switch (c)
	{
	case 'a': return 1;
	case 'b': return 2;
	default: break;
	}
	if (c == 'x' || c == 'y' || c == 'z')
		return 3;
	return 4;
}

int main()
{
	long h = 0;
	for (int c = -2; c < 300; ++c)
	{
		h = h * 3 + space(c) + 2 * not_digitish(static_cast<unsigned char>(c)) + 5 * classify(c - 10) +
			7 * weights(c % 7) + 11 * dispatch(c);
		h %= 1000000007;
	}
	std::printf("%ld\n", h);
	return 0;
}
