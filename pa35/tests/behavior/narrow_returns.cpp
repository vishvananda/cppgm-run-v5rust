// Narrow results returned without extending them (O1+: the psABI leaves the
// bits above a narrow return type undefined, so the caller extends): bool,
// signed and unsigned char and short, and int results computed in 64-bit
// registers, used by callers as indexes and in 64-bit arithmetic.
#include <cstdint>
#include <cstdio>

__attribute__((noinline)) bool odd_sum(long a, long b) { return ((a + b) & 1) != 0; }
__attribute__((noinline)) signed char low_signed(long x) { return static_cast<signed char>(x * 37); }
__attribute__((noinline)) unsigned char low_unsigned(long x) { return static_cast<unsigned char>(x * 41 + 3); }
__attribute__((noinline)) short low_short(long x) { return static_cast<short>(x * 1009); }
__attribute__((noinline)) unsigned short low_ushort(long x) { return static_cast<unsigned short>(x * 7919); }
__attribute__((noinline)) int low_int(long x) { return static_cast<int>(x * 0x10000001L); }

int main()
{
	static long table[256];
	for (int i = 0; i < 256; ++i)
		table[i] = i * 3;
	long h = 0;
	for (long x = -300; x < 300; x += 7)
	{
		h += odd_sum(x, x / 3) ? table[low_unsigned(x)] : -table[low_unsigned(x + 1)];
		h += static_cast<long>(low_signed(x)) * 100000 + low_short(x) + static_cast<long>(low_ushort(x)) * 3;
		h = h * 3 + static_cast<long>(low_int(x)) * 5;
		h %= 1000000000039L;
	}
	std::printf("%ld\n", h);
	return 0;
}
