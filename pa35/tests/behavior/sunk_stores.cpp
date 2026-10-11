// Stores to a slot sunk to the one successor that may read it (O2+
// memory forwarding): push_back's by-reference argument temporaries
// stored only on the growth path; a local whose address a call keeps,
// read back through that pointer on the other path (no sinking); one a
// later call may read (no sinking); one overwritten whole on the other
// path before any read.
#include <cstdio>
#include <cstdint>
#include <vector>
struct Pair
{
	uint16_t a;
	uint8_t b;
};
__attribute__((noinline)) void bytes(std::vector<uint8_t>& out, uint64_t value, unsigned n)
{
	for (unsigned k = 0; k < n; ++k)
		out.push_back(static_cast<uint8_t>(value >> (8 * k)));
}
__attribute__((noinline)) void pairs(std::vector<Pair>& out, unsigned n)
{
	for (unsigned k = 0; k < n; ++k)
		out.push_back(Pair{ static_cast<uint16_t>(k * 7), static_cast<uint8_t>(k) });
}
static const int* kept;
__attribute__((noinline)) void keep(const int& x)
{
	kept = &x;
}
__attribute__((noinline)) int read_back(int n)
{
	int v = 0, sum = 0;
	for (int i = 0; i < n; ++i)
	{
		v = i * 3 + 1;
		if (i % 4 == 0)
			keep(v);
		else
			sum += *kept;
	}
	return sum;
}
static int peeked;
__attribute__((noinline)) void peek()
{
	peeked += *kept;
}
__attribute__((noinline)) int later_call(int n)
{
	int v = 0;
	for (int i = 0; i < n; ++i)
	{
		v = i * 5;
		if (i == 0)
			keep(v);
		else
			peek();
	}
	return peeked;
}
__attribute__((noinline)) int overwritten(int n, int* out)
{
	int v = 0;
	int sum = 0;
	for (int i = 0; i < n; ++i)
	{
		v = i;
		if (i % 3 == 0)
		{
			keep(v);
			sum += *kept;
		}
		else
		{
			v = i * 2;
			sum += v;
		}
	}
	*out = v;
	return sum;
}
int main()
{
	std::vector<uint8_t> out;
	bytes(out, 0x1122334455667788ull, 8);
	bytes(out, 0xa5a5u, 2);
	unsigned check = 0;
	for (uint8_t c : out)
		check = check * 31 + c;
	std::vector<Pair> ps;
	pairs(ps, 20);
	unsigned pcheck = 0;
	for (const Pair& p : ps)
		pcheck = pcheck * 7 + p.a + p.b;
	int last = 0;
	const int o = overwritten(10, &last);
	std::printf("bytes %zu %u pairs %u read_back %d later %d overwritten %d %d\n", out.size(), check, pcheck, read_back(13),
		later_call(6), o, last);
	return 0;
}
