// 128-bit integer arithmetic as pairs of 64-bit halves (O2+): logic,
// addition and subtraction carrying across the halves, negation, shifts
// by constants on both sides of 64 and by variable counts, signed and
// unsigned compares deciding on either half, extensions and truncations,
// i128 locals in loops and phis, loads and stores through pointers, and
// multiplication and division kept whole between expanded operations
// (scalar_fold's normalize_integer and is_integer among them).
#include <cstdint>
#include <cstdio>
#include <initializer_list>
typedef unsigned __int128 u128;
typedef __int128 s128;
struct Literal
{
	uint64_t low, high;
};
static u128 to_u128(Literal b) { return (static_cast<u128>(b.high) << 64) | b.low; }
static Literal from_u128(u128 v)
{
	Literal b;
	b.low = static_cast<uint64_t>(v);
	b.high = static_cast<uint64_t>(v >> 64);
	return b;
}
static u128 mask_of(unsigned width) { return width >= 128 ? ~static_cast<u128>(0) : (static_cast<u128>(1) << width) - 1; }
static u128 extend(u128 value, unsigned width, bool sign)
{
	if (width >= 128)
		return value;
	value &= mask_of(width);
	if (sign && width > 0 && ((value >> (width - 1)) & 1) != 0)
		value |= ~mask_of(width);
	return value;
}
__attribute__((noinline)) Literal normalize(unsigned width, bool sign, Literal bits)
{
	return from_u128(extend(to_u128(bits), width, sign));
}
__attribute__((noinline)) bool is_value(unsigned width, bool sign, Literal c, int64_t value)
{
	Literal want;
	want.low = static_cast<uint64_t>(value);
	want.high = value < 0 ? ~0ull : 0;
	want = normalize(width, sign, want);
	const Literal have = normalize(width, sign, c);
	return have.low == want.low && have.high == want.high;
}
static uint64_t mix(uint64_t h, u128 v)
{
	h = (h ^ static_cast<uint64_t>(v)) * 0x100000001b3ull;
	return (h ^ static_cast<uint64_t>(v >> 64)) * 0x100000001b3ull;
}
__attribute__((noinline)) uint64_t arithmetic(u128 a, u128 b, unsigned k)
{
	uint64_t h = 0xcbf29ce484222325ull;
	h = mix(h, a + b);
	h = mix(h, a - b);
	h = mix(h, b - a);
	h = mix(h, a & b);
	h = mix(h, a | b);
	h = mix(h, a ^ b);
	h = mix(h, ~a);
	h = mix(h, -b);
	h = mix(h, a << 1);
	h = mix(h, a << 63);
	h = mix(h, a << 64);
	h = mix(h, a << 100);
	h = mix(h, b >> 1);
	h = mix(h, b >> 64);
	h = mix(h, b >> 127);
	h = mix(h, static_cast<u128>(static_cast<s128>(a) >> 3));
	h = mix(h, static_cast<u128>(static_cast<s128>(a) >> 70));
	h = mix(h, a << (k & 127));
	h = mix(h, b >> (k & 127));
	h = mix(h, static_cast<u128>(static_cast<s128>(b) >> (k & 127)));
	h = mix(h, a * b);
	if (b != 0)
		h = mix(h, a / b + a % b);
	h = h * 3 + (a < b) + 2 * (a <= b) + 4 * (a > b) + 8 * (a >= b) + 16 * (a == b) + 32 * (a != b);
	const s128 sa = static_cast<s128>(a), sb = static_cast<s128>(b);
	h = h * 3 + (sa < sb) + 2 * (sa <= sb) + 4 * (sa > sb) + 8 * (sa >= sb);
	h = h * 3 + static_cast<uint32_t>(a) + static_cast<uint8_t>(b >> 64) + static_cast<int16_t>(a >> 120);
	return h;
}
__attribute__((noinline)) u128 sum_loop(const uint64_t* xs, int n, int64_t bias)
{
	u128 total = static_cast<u128>(static_cast<s128>(bias));
	s128 signed_total = 0;
	for (int i = 0; i < n; ++i)
	{
		total += static_cast<u128>(xs[i]) << (i % 5);
		signed_total -= static_cast<s128>(static_cast<int64_t>(xs[i]));
		if (total > (static_cast<u128>(1) << 70))
			total >>= 3;
	}
	return total ^ static_cast<u128>(signed_total);
}
__attribute__((noinline)) void through_memory(u128* out, const u128* in, int n)
{
	for (int i = 0; i < n; ++i)
	{
		const u128 v = in[i];
		out[i] = v > 1000 ? v - 1000 : v + (static_cast<u128>(i) << 64);
	}
}
int main()
{
	const uint64_t samples[] = { 0, 1, 2, 0x7fffffffffffffffull, 0x8000000000000000ull, 0xffffffffffffffffull, 12345,
		0xdeadbeefcafef00dull };
	uint64_t h = 0;
	unsigned k = 0;
	for (uint64_t x : samples)
		for (uint64_t y : samples)
		{
			const u128 a = (static_cast<u128>(x) << 64) | y, b = (static_cast<u128>(y ^ 0x5555) << 64) | (x + 7);
			h = h * 31 + arithmetic(a, b, k += 13);
			h = h * 31 + arithmetic(y, x, k += 7);
		}
	unsigned norm = 0;
	for (unsigned width : { 1u, 8u, 16u, 32u, 63u, 64u, 65u, 127u, 128u })
		for (uint64_t x : samples)
		{
			Literal l = { x, ~x };
			const Literal n1 = normalize(width, false, l), n2 = normalize(width, true, l);
			norm = norm * 7 + static_cast<unsigned>(n1.low ^ n1.high ^ n2.low ^ (n2.high >> 3));
			norm = norm * 3 + is_value(width, true, l, -1) + 2 * is_value(width, false, l, 255) + 4 * is_value(width, true, n2, 0);
		}
	const u128 loop = sum_loop(samples, 8, -5);
	u128 in[6] = { 5, 1001, static_cast<u128>(1) << 100, ~static_cast<u128>(0), 999, 1000 }, out[6];
	through_memory(out, in, 6);
	uint64_t mem = 0;
	for (u128 v : out)
		mem = mix(mem, v);
	std::printf("%llx %x %llx %llx %llx\n", static_cast<unsigned long long>(h), norm, static_cast<unsigned long long>(loop),
		static_cast<unsigned long long>(loop >> 64), static_cast<unsigned long long>(mem));
	return 0;
}
