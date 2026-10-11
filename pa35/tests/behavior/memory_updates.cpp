// Loads, operations and stores of one place fused into one update (O1+
// over virtual registers: add/sub/and/or/xor [m], x): 8-, 16-, 32- and
// 64-bit fields and globals, immediates and registers, an indexed place,
// and a value still read after its store (not fused).
#include <cstdint>
#include <cstdio>

struct Counters
{
	uint8_t small;
	int16_t mid;
	uint32_t word;
	uint64_t wide;
	uint32_t hist[16];
};

uint64_t g_total;
int32_t g_flags = 0x0f0f;

__attribute__((noinline)) void bump(Counters& c, unsigned k, uint32_t x)
{
	c.small += 3;
	c.mid -= 7;
	c.word ^= x;
	c.wide += x;
	c.hist[k & 15] += 1;
	g_total += c.wide;
	g_flags &= ~static_cast<int32_t>(k);
	g_flags |= static_cast<int32_t>(x & 0x100);
}

__attribute__((noinline)) uint64_t again(Counters& c)
{
	c.wide += 5;
	return c.wide * 2;
}

int main()
{
	Counters c = {};
	for (uint32_t i = 0; i < 1000; ++i)
		bump(c, i * 7, i * 2654435761u);
	uint64_t h = again(c);
	for (uint32_t k : c.hist)
		h = h * 31 + k;
	std::printf("%u %d %u %llu %llu %d %llu\n", c.small, c.mid, c.word, static_cast<unsigned long long>(c.wide),
		static_cast<unsigned long long>(g_total), g_flags, static_cast<unsigned long long>(h));
	return 0;
}
