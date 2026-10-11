// Phis written on the edges out of conditional blocks (O1+: their edge
// blocks hold only what the phi's block takes, so a phi may share a
// register with a value live only toward the other successor): masks
// built in loops with values live on one edge only, accumulators updated
// on some paths, swaps through phis, and a switch feeding phis.
#include <cstdio>
#include <cstdint>
__attribute__((noinline)) uint32_t mask_for(const int32_t* a, const int32_t* b, int32_t last)
{
	uint32_t forbid = 0;
	for (unsigned r = 0; r < 32; ++r)
		if (a[r] < last || b[r] <= last)
			forbid |= 1u << r;
	return forbid;
}
__attribute__((noinline)) uint64_t paths(const uint8_t* s, unsigned n)
{
	uint64_t x = 1, y = 2, z = 3;
	for (unsigned i = 0; i < n; ++i)
	{
		const uint8_t c = s[i];
		if (c < 64)
		{
			if (c & 1)
				x = x * 3 + c;
			else
			{
				const uint64_t t = x;
				x = y;
				y = t + c;
			}
		}
		else if (c < 192)
			z ^= x + (c << 2);
		else
		{
			switch (c & 7)
			{
			case 0: y += z; break;
			case 3: z = y - x; break;
			case 5: x = z; z = y; y = c; break;
			default: break;
			}
		}
	}
	return x * 1000003 + y * 101 + z;
}
int main()
{
	int32_t a[32], b[32];
	for (int i = 0; i < 32; ++i)
	{
		a[i] = (i * 7) % 13 - 3;
		b[i] = (i * 5) % 11 - 2;
	}
	uint64_t sum = 0;
	for (int last = -4; last < 12; ++last)
		sum += mask_for(a, b, last);
	uint8_t s[1000];
	for (int i = 0; i < 1000; ++i)
		s[i] = static_cast<uint8_t>(i * 131 + (i >> 2));
	sum += paths(s, 1000);
	std::printf("%llu\n", static_cast<unsigned long long>(sum));
	return 0;
}
