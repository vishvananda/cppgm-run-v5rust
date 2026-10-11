// Unrolling of counted loops with branching bodies (host O3) applied to a
// loop that is its header alone (a do-while body and test in one block):
// each pass's copy goes on to the next pass's copy, never to itself.
#include <cstdio>

template <int N>
__attribute__((noinline)) int sum(const int* a)
{
	int s = 0, i = 0;
	do
	{
		s += a[i];
	} while (++i < N);
	return s;
}

__attribute__((noinline)) unsigned mixed(const unsigned* a, unsigned k)
{
	unsigned s = 0;
	int i = 0;
	do
	{
		unsigned x = a[i];
		s = (s ^ x) * k + (x >> 3) + (s << 5) - (x | k) + (s >> 7) * (x & 255);
		s = (s ^ (x * 7)) + (s >> 11) * (k | 3) - (x << 2) + (s & 0xff00) * (x ^ k);
	} while (++i < 3);
	return s;
}

int main(int argc, char**)
{
	int a[20];
	for (int i = 0; i < 20; ++i)
		a[i] = i * argc + 1;
	std::printf("%d %d %d %d %d\n", sum<2>(a), sum<5>(a), sum<8>(a), sum<16>(a), sum<17>(a));
	const unsigned u[3] = { 1, 2, 3 };
	std::printf("%u\n", mixed(u, 5));
	return 0;
}
