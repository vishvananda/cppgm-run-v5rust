// A truncation used only by compares and stores of its type stands for its
// source register (O1+), but not when it is the stored value of a slot the
// selector promotes (one store in the entry): the slot's loads, here in a
// handler, read the truncated value.
#include <cstdint>
#include <cstdio>

volatile uint64_t vx = 0xF000001FFull;
volatile int vt = 1;

__attribute__((noinline)) void thrower()
{
	if (vt)
		throw 1;
}

__attribute__((noinline)) int byte_value(uint64_t x)
{
	uint8_t b;
	try
	{
		b = static_cast<uint8_t>(x);
		thrower();
	}
	catch (...)
	{
		return b * 3;
	}
	return -1;
}

__attribute__((noinline)) long word_value(uint64_t x)
{
	int32_t n;
	try
	{
		n = static_cast<int32_t>(x);
		thrower();
	}
	catch (...)
	{
		return static_cast<long>(n) + 1;
	}
	return -1;
}

__attribute__((noinline)) long check(long x)
{
	if (x == 12345)
		throw 1;
	return x;
}

__attribute__((noinline)) long short_value(unsigned long p0, int p2)
{
	short v = static_cast<short>(p0);
	try
	{
		long w = p2;
		if (static_cast<unsigned long>(w) < static_cast<unsigned long>(static_cast<__int128>(w)))
			v = 7;
		check(w);
	}
	catch (...)
	{
	}
	return v;
}

int main(int argc, char**)
{
	std::printf("%d %ld\n", byte_value(vx), word_value(vx));
	for (long k = 0; k < 4; ++k)
		std::printf("%ld\n", short_value(0x12345678abcdefull * (k + argc), static_cast<int>(k)));
	return 0;
}
