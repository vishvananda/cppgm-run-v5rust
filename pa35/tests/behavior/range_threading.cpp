// Branches decided by an earlier range test of the same value, through
// integer extensions (O2+ jump threading): a reader whose fast path
// returns a byte below 0x80 and not a backslash, its slow path anything,
// merged before the loop header tests the character again; signed bytes
// widened by sign extension; unsigned and signed bounds on both sides.
#include <cstdio>
#include <cstdint>
struct Reader
{
	const unsigned char* text;
	unsigned size, pos;
	int cur;
	__attribute__((noinline)) int slow(unsigned at)
	{
		if (at >= size)
			return -1;
		const unsigned char b = text[at];
		return b == '\\' ? 0x5c00 + (at & 7) : b >= 0x80 ? 0x10000 + b : b == '?' ? -2 : b;
	}
	void advance()
	{
		const unsigned at = pos + 1;
		pos = at;
		if (at < size && text[at] < 0x80 && text[at] != '\\' && text[at] != '?')
			cur = text[at];
		else
			cur = slow(at);
	}
};
__attribute__((noinline)) unsigned scan(Reader& r, unsigned& specials)
{
	unsigned letters = 0;
	for (;;)
	{
		const int c = r.cur;
		if (c == -1)
			break;
		if (c == '\\')
			specials += 7;
		else if (c < 0x80 && c >= 0)
		{
			if ((c >= 'a' && c <= 'z') || c == '_')
				++letters;
			else if (c < 0)
				specials += 1000;
		}
		else if (c == -2)
			specials += 3;
		else
			specials += static_cast<unsigned>(c) & 0xff;
		r.advance();
	}
	return letters;
}
__attribute__((noinline)) int widened(const signed char* s, int n)
{
	int total = 0;
	for (int i = 0; i < n; ++i)
	{
		const signed char b = s[i];
		int c = b;
		if (b >= 0)
		{
			if (c < 0)
				total += 100000;
			if (c > 126)
				total += 1000;
			total += c;
		}
		else
		{
			const unsigned u = static_cast<unsigned char>(b);
			if (u < 0x80)
				total += 50000;
			if (c >= -128 && c <= -1)
				total -= c;
		}
		const uint16_t w = static_cast<uint16_t>(static_cast<unsigned char>(b));
		uint32_t z = w;
		if (w < 300 && z > 255)
			total += 77777;
	}
	return total;
}
int main()
{
	unsigned char text[600];
	for (unsigned i = 0; i < sizeof text; ++i)
		text[i] = static_cast<unsigned char>((i * 37 + (i >> 3) * 11) ^ (i & 0x40 ? 0x80 : 0));
	text[17] = '\\';
	text[18] = '?';
	text[99] = 0x5c;
	Reader r{ text, sizeof text, 0, text[0] };
	unsigned specials = 0;
	const unsigned letters = scan(r, specials);
	signed char s[256];
	for (int i = 0; i < 256; ++i)
		s[i] = static_cast<signed char>(i * 13);
	std::printf("%u %u %d\n", letters, specials, widened(s, 256));
	return 0;
}
