// Registers live across one rarely taken call kept in caller-saved
// registers, stored before the call and loaded after it (host O2+): a
// vector's growth path, values that are also the call's arguments, values
// live across two such calls (kept in callee-saved registers), and a call
// with a handler (its values stay in callee-saved registers).
#include <cstdio>
#include <stdexcept>
#include <vector>

struct Sink
{
	std::vector<unsigned> items;
	unsigned long sum = 0;
	__attribute__((noinline)) void add(unsigned u, unsigned v)
	{
		if (u == v)
			return;
		items.push_back(u * 31 + v);
		sum += static_cast<unsigned long>(u) * 7 + v;
	}
};

__attribute__((noinline)) long helper(long x) { return x * 3 + 1; }
__attribute__((noinline)) long risky(long x)
{
	if (x % 17 == 3)
		throw std::runtime_error("risky");
	return x - 1;
}

__attribute__((noinline)) long twice(long a, long b, bool c, bool d)
{
	long t = a * b;
	if (c)
		t += helper(a);
	if (d)
		t ^= helper(b + t);
	return t + a - b;
}

__attribute__((noinline)) long guarded(long a, long b, bool c)
{
	long t = a + b;
	if (c)
	{
		try
		{
			t += risky(a);
		}
		catch (const std::exception&)
		{
			t += a * b;
		}
	}
	return t * 2 + a;
}

__attribute__((noinline)) long argument(long a, bool c)
{
	long t = a;
	if (c)
		t = helper(a) + a;
	return t + a;
}

int main()
{
	Sink s;
	for (unsigned u = 0; u < 300; ++u)
		s.add(u % 50, (u * 7) % 50);
	unsigned long h = s.sum;
	for (unsigned k : s.items)
		h = h * 31 + k;
	for (long i = 0; i < 100; ++i)
	{
		h += twice(i, i + 2, i & 1, i % 3 == 0);
		h += guarded(i, 5, i % 4 == 0);
		h += argument(i, i % 5 == 1);
		h %= 1000000007;
	}
	std::printf("%lu\n", h);
	return 0;
}
