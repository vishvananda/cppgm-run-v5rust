// Loads a branch tests compared with zero in memory (O1+: `cmp $0, m`):
// bool, signed char, 16-, 32- and 64-bit and pointer fields and an element
// through an index, a volatile flag (read as written), and a load whose
// value is also used after the test.
#include <cstdint>
#include <cstdio>

struct Fields
{
	bool flag;
	signed char tiny;
	uint16_t half;
	uint32_t word;
	int64_t wide;
	const Fields* next;
	uint8_t items[8];
};

__attribute__((noinline)) int64_t branches(const Fields* f, unsigned k, int64_t x)
{
	if (f->flag)
		x += 3;
	if (f->tiny)
		x ^= 0x55;
	if (f->half)
		x *= 5;
	if (f->word)
		x -= 7;
	if (f->wide)
		x += f->wide >> 60;
	if (f->next)
		x *= 2;
	if (f->items[k & 7])
		x += 11;
	return x;
}

volatile int g_ready = 0;

__attribute__((noinline)) int volatile_flag()
{
	int seen = 0;
	if (g_ready)
		++seen;
	g_ready = 1;
	if (g_ready)
		seen += 2;
	return seen;
}

__attribute__((noinline)) uint32_t reused(const Fields* f)
{
	const uint32_t w = f->word;
	if (w)
		return w * 3;
	return 99;
}

int main()
{
	Fields a = { false, 0, 0, 0, 0, nullptr, { 0, 1, 0, 2, 0, 0, 3, 0 } };
	Fields b = { true, -1, 7, 9, -5, &a, { 1, 0, 1, 0, 1, 0, 1, 0 } };
	int64_t h = 0;
	for (unsigned k = 0; k < 8; ++k)
		h = h * 31 + branches(&a, k, k) + branches(&b, k, -static_cast<int64_t>(k));
	h = h * 31 + volatile_flag() + reused(&a) + reused(&b);
	std::printf("%lld\n", static_cast<long long>(h));
	return 0;
}
