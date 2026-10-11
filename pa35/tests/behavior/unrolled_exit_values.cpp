// Unrolled loops (host O3, branching bodies): code after the loop reads a
// header phi as the last test's copy read it. A latch value the header
// itself computes (a single-block do-while's step, or `prev = cur` reading
// the other phi) is not stepped once more for the exit.
#include <cstdio>

volatile int vk = 1;

__attribute__((noinline)) int countdown(int k)
{
	int s = 0, t = 0;
	int i = 4;
	do
	{
		switch ((s >> 1) & 7)
		{
		case 1:
			if (((t - k) & 15) == 6)
				return 99;
			break;
		case 0:
			if (((s >> 1) & 7) == 7)
				s += 1;
			break;
		default:
			t ^= i;
		}
	} while ((i -= 1) > -13);
	return s + t * 3 + i;
}

__attribute__((noinline)) long previous(long a, long b)
{
	long prev = 78, cur = a;
	for (unsigned i = 0; i < 2; ++i)
	{
		prev = cur;
		if (b & 1)
			cur = cur * 3 + b;
	}
	return prev;
}

int main(int argc, char**)
{
	std::printf("%d\n", countdown(vk));
	for (long b = 0; b < 4; ++b)
		std::printf("%ld\n", previous(argc + 10, b));
	return 0;
}
