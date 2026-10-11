// Fields updated in place (O1+: load, operate, store back as one
// operation on memory): counters of each width, pointer bumps as an
// inlined push_back's, and/or/xor/sub with registers and immediates,
// an operand loaded through a pointer to the same field, and a value
// read again after its update; also a multiply by a power of two at 32
// bits (a shift).
#include <cstdio>
#include <cstdint>
#include <vector>
struct Counters { uint8_t b; int8_t sb; uint16_t h; int32_t i; uint32_t u; int64_t l; uint64_t q; unsigned char* cur; };
__attribute__((noinline)) void bump(Counters* c, int32_t k, uint64_t m, const int32_t* alias)
{
	c->b += 7;
	c->sb -= 3;
	c->h += 1000;
	c->i -= k;
	c->u |= static_cast<uint32_t>(m);
	c->u ^= 0x55u;
	c->l += -9;
	c->q &= m | 0xf0f0;
	c->cur += 3;
	c->i += *alias;
}
__attribute__((noinline)) uint32_t bytes(std::vector<uint8_t>& out, uint64_t value, unsigned n)
{
	for (unsigned k = 0; k < n; ++k)
		out.push_back(static_cast<uint8_t>(value >> (8 * k)));
	return static_cast<uint32_t>(out.size());
}
__attribute__((noinline)) uint32_t scaled(uint32_t x, uint32_t y) { return x * 8 + y * 16 + (x ^ y) * 2; }
int main()
{
	unsigned char buffer[64] = {};
	Counters c{ 250, -126, 65000, 5, 0x1234, 3, ~0ull, buffer };
	uint64_t sum = 0;
	for (int r = 0; r < 100; ++r)
	{
		if (c.i > (1 << 20) || c.i < -(1 << 20))
			c.i = r;
		bump(&c, r, 0x10203ull * r, &c.i);
		if (c.cur >= buffer + 60)
			c.cur = buffer;
		sum += c.b + c.sb + c.h + c.i + c.u + static_cast<uint64_t>(c.l) + c.q + static_cast<uint64_t>(c.cur - buffer);
	}
	std::vector<uint8_t> out;
	uint32_t n = 0;
	for (unsigned k = 0; k < 300; ++k)
		n += bytes(out, 0x0102030405060708ull * k, k % 9);
	for (uint8_t v : out)
		sum = sum * 31 + v;
	for (uint32_t k = 0; k < 50; ++k)
		sum += scaled(k * 0x9e3779b9u, k);
	std::printf("%llu %u %u\n", static_cast<unsigned long long>(sum), n, static_cast<unsigned>(out.size()));
	return 0;
}
