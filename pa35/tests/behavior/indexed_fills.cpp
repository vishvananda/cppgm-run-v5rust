// Counted loops storing literals or copying elements over fixed local
// ranges (O2+ indexed_stores: zeroinit, repeated-byte stores, copyobj),
// some entered from a branching block (a preheader made first), next to
// loops that must stay (a running sum, a short array tail).
#include <climits>
#include <cstdio>
__attribute__((noinline)) int fills(int k)
{
	int writer[32];
	for (unsigned r = 0; r < 32; ++r)
		writer[r] = -1;
	int nc[32], ne[32];
	for (unsigned r = 0; r < 32; ++r)
		nc[r] = ne[r] = INT_MAX;
	unsigned first[33] = { 0 };
	for (unsigned r = 0; r < 32; ++r)
		first[r + 1] = first[r] + r * static_cast<unsigned>(k);
	unsigned copy[32];
	for (unsigned r = 0; r < 32; ++r)
		copy[r] = first[r];
	short tail[10];
	for (int r = 3; r < 10; ++r)
		tail[r] = 0x1234;
	signed char bytes[24];
	for (int r = 0; r < 24; ++r)
		bytes[r] = 0;
	long wide[5];
	for (int r = 1; r < 5; ++r)
		wide[r] = 0x123456789abcdefL;
	writer[k & 31] = k;
	nc[(k + 1) & 31] = 5;
	bytes[k % 24] = 7;
	unsigned s = 0;
	for (int r = 0; r < 32; ++r)
		s += static_cast<unsigned>(writer[r]) * 3 + static_cast<unsigned>(nc[r]) + static_cast<unsigned>(ne[r]) + copy[r];
	for (int r = 3; r < 10; ++r)
		s += tail[r];
	for (int r = 0; r < 24; ++r)
		s += bytes[r];
	for (int r = 1; r < 5; ++r)
		s += static_cast<int>(wide[r] >> (8 * r));
	return static_cast<int>(s);
}
int main()
{
	long t = 0;
	for (int k = 0; k < 200; ++k)
		t += fills(k);
	std::printf("%ld\n", t);
	return 0;
}
