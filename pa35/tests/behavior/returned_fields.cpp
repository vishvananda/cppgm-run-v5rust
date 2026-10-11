// A struct filled by a copy from an array on one path and returned into
// it by an out-of-line call on the other, then read by fields (O2+ slot
// splitting: the call fills a temporary its fields read back), as the
// parser's token peeks; also read whole afterwards, and passed on.
#include <cstdio>
#include <vector>
struct Tok { unsigned char kind, simple, flags; unsigned location; const char* text; unsigned length; const void* id; };
struct Stream {
	std::vector<Tok> window;
	std::vector<Tok> replay;
	unsigned position = 0;
	bool replaying = false;
	Tok slow(unsigned ahead) const;
	Tok peek(unsigned ahead = 0) const { return replaying ? slow(ahead) : window[(position + ahead) % window.size()]; }
};
__attribute__((noinline)) Tok Stream::slow(unsigned ahead) const { Tok t = replay[(position + ahead) % replay.size()]; t.flags ^= 1; return t; }
__attribute__((noinline)) unsigned sink(const Tok& t) { return t.location + t.length; }
__attribute__((noinline)) unsigned scan(Stream& s, unsigned rounds)
{
	unsigned r = 0;
	for (unsigned i = 0; i < rounds; ++i)
	{
		s.replaying = (i % 3) == 0;
		if (s.peek().kind == 0 && s.peek().simple == (i & 7))
			r += 3;
		const Tok t = s.peek(1);
		if (t.kind == 2)
			r += t.location;
		if ((i & 15) == 0)
			r += sink(t) + t.flags;
		s.position += 1;
	}
	return r;
}
int main()
{
	Stream s;
	for (unsigned i = 0; i < 37; ++i)
		s.window.push_back(Tok{ static_cast<unsigned char>(i % 3), static_cast<unsigned char>(i % 8), 0, i * 7, "w", i, nullptr });
	for (unsigned i = 0; i < 29; ++i)
		s.replay.push_back(Tok{ static_cast<unsigned char>((i + 1) % 3), static_cast<unsigned char>(i % 5), 2, i * 11, "r", i + 3, nullptr });
	std::printf("%u\n", scan(s, 5000));
	return 0;
}
