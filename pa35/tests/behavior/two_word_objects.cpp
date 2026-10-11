// 12- and 16-byte objects kept in two register words (host objects: a slot
// taken whole or by fields within a word becomes an i64 and a u32 or i64):
// a value built by fields on one path and returned by a call on another,
// copied into a member; field updates in the second word; zero
// initialization; and copies between such locals.
#include <cstdio>
#include <cstdint>
struct Char { int32_t code; uint32_t begin; uint32_t end; };
struct Wide { uint64_t low; uint16_t a; uint8_t b; int8_t c; uint32_t d; };
__attribute__((noinline)) Char slow(const unsigned char* bytes, uint32_t at)
{
	Char c;
	c.code = bytes[at] * 3 + 0x10000;
	c.begin = at;
	c.end = at + 2;
	return c;
}
struct Lexer
{
	const unsigned char* bytes;
	uint32_t size;
	Char cur;
	__attribute__((noinline)) void advance(uint32_t at)
	{
		Char c;
		if (at < size && bytes[at] < 0x80 && bytes[at] != '\\')
		{
			c.code = bytes[at];
			c.begin = at;
			c.end = at + 1;
		}
		else
			c = slow(bytes, at);
		cur = c;
	}
};
__attribute__((noinline)) Wide bump(Wide w, int k)
{
	Wide x = {};
	if (k & 1)
		x = w;
	x.a = static_cast<uint16_t>(x.a + k * 1000);
	x.b = static_cast<uint8_t>(x.b ^ k);
	x.c = static_cast<int8_t>(-x.c - k);
	x.d += static_cast<uint32_t>(k) * 0x01010101u;
	x.low += x.d;
	return x;
}
int main()
{
	unsigned char text[64];
	for (int i = 0; i < 64; ++i)
		text[i] = static_cast<unsigned char>(i * 37 % 200);
	text[10] = '\\';
	Lexer lex = { text, 60, { 0, 0, 0 } };
	uint64_t h = 0;
	for (uint32_t at = 0; at < 64; ++at)
	{
		lex.advance(at);
		h = h * 31 + static_cast<uint32_t>(lex.cur.code) + lex.cur.begin * 7 + lex.cur.end * 13;
	}
	Wide w = { 5, 6, 7, -8, 9 };
	for (int k = 0; k < 9; ++k)
	{
		w = bump(w, k);
		h = h * 31 + w.low + w.a + w.b + static_cast<uint64_t>(static_cast<int64_t>(w.c)) + w.d;
	}
	std::printf("%llu\n", static_cast<unsigned long long>(h));
	return 0;
}
