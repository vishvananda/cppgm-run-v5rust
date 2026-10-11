// Packed eightbytes whose padding no write reaches (merged without the old
// bytes) and the simplifier's masks of values with known-zero bits.
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <vector>

struct Key
{
	uint8_t op;
	uint8_t sub;
	uint64_t value;
};

struct Pair
{
	uint8_t kind;
	uint8_t flag;
	uint32_t index;
};

__attribute__((noinline)) uint64_t keys(const uint8_t* ops, unsigned n)
{
	std::vector<Key> seen;
	uint64_t h = 0;
	for (unsigned i = 0; i < n; ++i)
	{
		Key key;
		key.op = ops[i];
		if (ops[i] & 1)
			key.sub = static_cast<uint8_t>(ops[i] * 3);
		else
			key.sub = 7;
		key.value = i * 0x10001ull;
		seen.push_back(key);
		h = h * 31 + key.op + (key.sub << 8) + key.value;
	}
	for (const Key& k : seen)
		h = h * 17 + k.op * 5 + k.sub + (k.value >> 3);
	return h;
}

__attribute__((noinline)) uint64_t pairs(const uint32_t* xs, unsigned n)
{
	Pair last = { 0, 0, 0 };
	uint64_t h = 0;
	for (unsigned i = 0; i < n; ++i)
	{
		Pair p;
		p.kind = static_cast<uint8_t>(xs[i]);
		p.flag = xs[i] > 100;
		p.index = xs[i] * 7;
		if (xs[i] % 3 == 0)
			last = p;
		h += last.kind + last.flag * 3 + last.index;
	}
	return h;
}

// Masks over or/and/shift chains: some fold, some must not.
__attribute__((noinline)) uint64_t masks(uint64_t x, uint8_t y, uint16_t z, int32_t s)
{
	const uint64_t a = ((x & 0xff00) | y) & 0xff;                 // y
	const uint64_t b = ((x & 0xff00) | y) & 0xffff;               // both parts
	const uint64_t c = ((static_cast<uint64_t>(z) << 16) | y) & 0xffff;   // y
	const uint64_t d = ((x >> 56) | (static_cast<uint64_t>(y) << 8)) & 0xff;   // x's top byte
	const uint32_t e = (static_cast<uint32_t>(s) | 0x80000000u) & 0x7fffffffu;
	const int32_t f = (s & -256) | static_cast<uint8_t>(y);
	const int32_t g = f & 0xff;
	const uint64_t h = (x | static_cast<uint64_t>(z)) & 0xffff;   // overlaps: must not drop x
	const uint16_t k = static_cast<uint16_t>((static_cast<uint16_t>(z & 0xf0) | y) & 0x0f);
	return a + (b << 1) + (c << 2) + (d << 3) + (static_cast<uint64_t>(e) << 4) + static_cast<uint64_t>(static_cast<int64_t>(g)) +
		(h << 5) + (static_cast<uint64_t>(k) << 7);
}

int main()
{
	uint8_t ops[64];
	uint32_t xs[64];
	for (unsigned i = 0; i < 64; ++i)
	{
		ops[i] = static_cast<uint8_t>(i * 37 + 11);
		xs[i] = i * 53 + 1;
	}
	std::printf("%llu\n", static_cast<unsigned long long>(keys(ops, 64)));
	std::printf("%llu\n", static_cast<unsigned long long>(pairs(xs, 64)));
	uint64_t acc = 0;
	for (unsigned i = 0; i < 40; ++i)
		acc = acc * 1000003 + masks(0x0123456789abcdefull * (i + 1), static_cast<uint8_t>(i * 29), static_cast<uint16_t>(i * 4099),
			static_cast<int32_t>(i * 0x9e3779b9u));
	std::printf("%llu\n", static_cast<unsigned long long>(acc));
	return 0;
}
