// Truncations read only through their low bits (O1+: they stand for
// their source): 32-bit adds, subs and muls of a 64-bit value's low half,
// narrow stores and compares, narrower truncations; and truncations that
// must stay (extended again, a phi, a shift, a branch, an index).
#include <cstdio>
#include <cstdint>
__attribute__((noinline)) uint32_t mix(uint32_t h, uint64_t v)
{
	h ^= static_cast<uint32_t>(v) + 0x9e3779b9u + (h << 6) + (h >> 2);
	h ^= static_cast<uint32_t>(v >> 32) + 0x9e3779b9u + (h << 6) + (h >> 2);
	return h;
}
__attribute__((noinline)) int narrow(uint64_t v, uint8_t* out, uint16_t* half)
{
	const uint32_t w = static_cast<uint32_t>(v);
	out[0] = static_cast<uint8_t>(w);
	half[0] = static_cast<uint16_t>(v * 3);
	const int32_t s = static_cast<int32_t>(v);
	int r = static_cast<uint32_t>(v >> 3) == 0x12345u ? 1 : 0;
	r += s < -5 ? 2 : 0;
	r += static_cast<int16_t>(v) == -2 ? 4 : 0;
	return r + static_cast<int32_t>(static_cast<uint32_t>(v) * 5u);
}
__attribute__((noinline)) uint64_t kept(uint64_t v, const uint32_t* table, int n)
{
	const uint32_t w = static_cast<uint32_t>(v);
	uint64_t again = w;                  // extended again
	uint32_t acc = w;
	for (int i = 0; i < n; ++i)
		acc = acc * 7 + static_cast<uint32_t>(v >> i);   // a phi
	const uint32_t shifted = static_cast<uint32_t>(v) >> 5;
	const uint32_t index = static_cast<uint32_t>(v) & 15;
	if (static_cast<uint32_t>(v >> 40))
		again += 3;
	return again + acc + shifted + table[index];
}
int main()
{
	uint64_t sum = 0;
	uint8_t b = 0;
	uint16_t h = 0;
	uint32_t table[16];
	for (int i = 0; i < 16; ++i)
		table[i] = static_cast<uint32_t>(i * 977);
	const uint64_t probes[] = { 0, 1, 0xffffffffull, 0x100000000ull, 0x123456789abcdef0ull, 0x91a28ull, 0xfffffffffffffffeull,
		0x8000000000000000ull, 0x7fffffffull, 0x80000000ull, 0xdeadbeefcafef00dull };
	for (uint64_t v : probes)
	{
		sum = sum * 31 + mix(static_cast<uint32_t>(sum), v);
		sum = sum * 31 + static_cast<uint32_t>(narrow(v, &b, &h)) + b + h;
		sum = sum * 31 + kept(v, table, static_cast<int>(v & 7));
	}
	std::printf("%llu\n", static_cast<unsigned long long>(sum));
	return 0;
}
