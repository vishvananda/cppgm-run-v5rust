// Pointer differences of elements whose size is not a power of two (O1+:
// an exact division, a shift and a multiplication by the inverse of the
// size's odd part): positive and negative, sizes with and without a power
// of two in them, and a vector's size.
#include <cstdio>
#include <vector>

template <unsigned N> struct Bytes { unsigned char b[N]; };

template <unsigned N>
__attribute__((noinline)) long span(const Bytes<N>* p, const Bytes<N>* q) { return p - q; }

template <unsigned N>
unsigned long check(long seed)
{
	static Bytes<N> items[50];
	unsigned long total = 0;
	for (int i = 0; i < 50; i += 7)
		for (int j = 0; j < 50; j += 3)
			total = total * 3 + span<N>(&items[i], &items[j]) + seed;
	return total;
}

struct Entry { long a, b, c; };

int main()
{
	unsigned long h = 0;
	h = h * 7 + check<3>(1);
	h = h * 7 + check<6>(2);
	h = h * 7 + check<7>(3);
	h = h * 7 + check<12>(4);
	h = h * 7 + check<24>(5);
	h = h * 7 + check<40>(6);
	h = h * 7 + check<56>(7);
	h = h * 7 + check<96>(8);
	h = h * 7 + check<144>(9);
	h = h * 7 + check<1001>(10);
	std::vector<Entry> v;
	unsigned long sizes = 0;
	for (int i = 0; i < 100; ++i)
	{
		v.push_back(Entry{ i, i * 2, i * 3 });
		sizes += v.size();
	}
	std::printf("%lu %lu\n", h, sizes);
	return 0;
}
