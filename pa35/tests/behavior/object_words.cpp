// 8-byte objects kept in a register word (host objects: a slot taken whole
// or by integer fields becomes an i64): fields of every width and
// signedness read and written at their offsets (a write masks the field
// in), whole copies from parameters, memory and calls, returns, pointer
// fields, and objects compared at a narrow field.
#include <cstdio>
#include <cstdint>
struct V { uint8_t kind; uint32_t index; };
struct S { int8_t a; int16_t b; int32_t c; };
struct H { uint16_t lo; uint8_t mid; int8_t sign; uint32_t hi; };
struct P { const char* p; };
__attribute__((noinline)) V next_of(const V* table, V v)
{
	while (v.kind == 1 && table[v.index].kind != 0)
		v = table[v.index];
	return v;
}
__attribute__((noinline)) S bump(S s, int k)
{
	s.a = static_cast<int8_t>(s.a - k);
	if (k & 1)
		s.b = static_cast<int16_t>(s.b * -3);
	s.c += s.a + s.b;
	return s;
}
__attribute__((noinline)) H mix(H h, uint32_t x)
{
	H r = h;
	r.mid = static_cast<uint8_t>(r.mid + x);
	r.sign = static_cast<int8_t>(-r.sign);
	if (x > 3)
		r.lo = static_cast<uint16_t>(r.lo ^ 0xffffu);
	r.hi = r.hi * 33u + r.lo + static_cast<uint32_t>(r.sign);
	return r;
}
__attribute__((noinline)) V make(uint8_t kind, uint32_t index)
{
	V v;
	v.kind = kind;
	v.index = index;
	return v;
}
__attribute__((noinline)) int length(P p)
{
	int n = 0;
	while (p.p[n] != 0)
		++n;
	return n;
}
int main()
{
	V table[8];
	for (int i = 0; i < 8; ++i)
		table[i] = make(static_cast<uint8_t>(i % 3 == 0 ? 0 : 1), static_cast<uint32_t>((i * 5 + 3) % 8));
	uint64_t h = 0;
	for (int i = 0; i < 8; ++i)
	{
		V r = next_of(table, make(1, static_cast<uint32_t>(i)));
		h = h * 31 + r.kind * 100 + r.index;
	}
	for (int k = -4; k < 5; ++k)
	{
		S s = { static_cast<int8_t>(k * 30), static_cast<int16_t>(k * 1000 - 7), k * 100000 };
		S t = bump(s, k);
		h = h * 31 + static_cast<uint64_t>(static_cast<int64_t>(t.a) + t.b * 7 + t.c);
		H x = { static_cast<uint16_t>(k * 4111), static_cast<uint8_t>(k + 250), static_cast<int8_t>(k * 31), static_cast<uint32_t>(k) * 0x01010101u };
		H y = mix(x, static_cast<uint32_t>(k + 4));
		h = h * 31 + y.lo + y.mid * 3u + static_cast<uint64_t>(static_cast<int64_t>(y.sign)) + y.hi;
	}
	P p = { "object words" };
	h = h * 31 + static_cast<uint64_t>(length(p));
	std::printf("%llu\n", static_cast<unsigned long long>(h));
	return 0;
}
