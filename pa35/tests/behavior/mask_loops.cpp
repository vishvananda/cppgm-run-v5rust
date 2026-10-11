// Counted loops of up to 16 passes whose tests depend on the induction and
// literals alone (O3: unrolled like table searches, each copy's test a
// constant): a caller-saved mask over 16 registers, a mask with the top
// bit, a shifted mask compared against a literal, and a loop whose test
// also reads data (not folded: the same answers either way).
#include <cstdio>
const unsigned kMask = 0x0fc7;
__attribute__((noinline)) void drop_masked(int* holds)
{
	for (unsigned r = 0; r < 16; ++r)
		if (kMask & (1u << r))
			holds[r] = -1;
}
__attribute__((noinline)) unsigned top_bits(const unsigned* v)
{
	unsigned s = 0;
	for (int r = 0; r < 16; ++r)
		if ((0x8001u >> r) & 1)
			s += v[r];
	return s;
}
__attribute__((noinline)) int compared(int* a)
{
	int n = 0;
	for (unsigned k = 0; k < 12; ++k)
		if (((k * 3) & 7) == 5)
		{
			a[k] += 10;
			++n;
		}
	return n;
}
__attribute__((noinline)) int mixed(const int* holds, int lo)
{
	int c = 0;
	for (unsigned r = 0; r < 16; ++r)
		if ((kMask & (1u << r)) && holds[r] > lo)
			++c;
	return c;
}
int main()
{
	int holds[16];
	unsigned v[16];
	int a[12];
	for (int i = 0; i < 16; ++i)
	{
		holds[i] = i * 7 - 20;
		v[i] = static_cast<unsigned>(i * i + 1);
	}
	for (int i = 0; i < 12; ++i)
		a[i] = i;
	const int before = mixed(holds, 0);
	drop_masked(holds);
	const int after = mixed(holds, -5);
	const int n = compared(a);
	std::printf("%d %d %u %d", before, after, top_bits(v), n);
	for (int i = 0; i < 16; ++i)
		std::printf(" %d", holds[i]);
	for (int i = 0; i < 12; ++i)
		std::printf(" %d", a[i]);
	std::printf("\n");
	return 0;
}
