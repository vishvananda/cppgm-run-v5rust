// Full unrolling of call-free 16-pass loops with branching bodies, several
// per function, including loops entered from a branching block (they get a
// preheader first) and loops whose values are read after them.
#include <cstdint>
#include <cstdio>

typedef uint8_t Reg;
const int32_t kAny = 0x7fffffff, kNothing = -1;

__attribute__((noinline)) int32_t meet(int32_t* holds, const int32_t* other, int n)
{
	int32_t sum = 0;
	for (int i = 0; i < n; ++i)
	{
		if (i & 1)
			for (Reg r = 0; r < 16; ++r)
				holds[r] = holds[r] == kAny || holds[r] == other[r] ? other[r] : kNothing;
		else
			for (Reg r = 0; r < 16; ++r)
				if (holds[r] != kNothing && holds[r] < other[i & 15])
					holds[r] = kNothing;
		for (Reg r = 0; r < 16; ++r)
			sum += holds[r] == kNothing ? 1 : holds[r] & 7;
	}
	return sum;
}

__attribute__((noinline)) int find_first(const int32_t* xs, int32_t v)
{
	Reg r = 0;
	for (; r < 16; ++r)
		if (xs[r] == v)
			break;
	return r;
}

__attribute__((noinline)) uint32_t masks(uint32_t used, const int32_t* weights)
{
	uint32_t best = 0, picked = 0;
	for (unsigned r = 0; r < 16; ++r)
		if (!(used & (1u << r)) && static_cast<uint32_t>(weights[r]) > best)
		{
			best = static_cast<uint32_t>(weights[r]);
			picked = r;
		}
	for (unsigned r = 0; r < 16; ++r)
		if (weights[r] < 0)
			used |= 1u << r;
	return picked * 65536 + used;
}

int main()
{
	int32_t holds[16], other[16];
	for (int r = 0; r < 16; ++r)
	{
		holds[r] = r % 3 == 0 ? kAny : r * 5;
		other[r] = r * 5 + (r & 1);
	}
	std::printf("%d\n", meet(holds, other, 9));
	for (int r = 0; r < 16; ++r)
		std::printf("%d ", holds[r]);
	std::printf("\n%d %d %d\n", find_first(other, 26), find_first(other, 25), find_first(other, -4));
	int32_t w[16];
	for (int r = 0; r < 16; ++r)
		w[r] = (r * 37) % 23 - 4;
	std::printf("%u %u\n", masks(0x0f0fu, w), masks(0, w));
	return 0;
}
