// Equality memcmp against literals feeding branches (O2+: one chunk tested,
// then the second in a new block): keyword tables matched by length then
// bytes, results used in phis, both polarities, chunks that share loads
// across candidates, a byte stored between candidates.
#include <cstdio>
#include <cstring>
struct Entry
{
	const char* text;
	unsigned length;
	int value;
};
static const Entry kWords[] = { { "new", 3, 1 }, { "and", 3, 2 }, { "not", 3, 3 }, { "xor", 3, 4 }, { "delete", 6, 5 },
	{ "and_eq", 6, 6 }, { "bitor", 5, 7 }, { "compl", 5, 8 }, { "or", 2, 9 }, { "not_eq", 6, 10 }, { "xor_eq", 6, 11 } };
__attribute__((noinline)) int lookup(const char* s, unsigned n)
{
	for (const Entry& e : kWords)
		if (e.length == n && std::memcmp(e.text, s, n) == 0)
			return e.value;
	return 0;
}
__attribute__((noinline)) int differs(const char* s, unsigned n)
{
	int score = 0;
	if (n == 4 && std::memcmp(s, "char", 4) != 0)
		score += 3;
	if (n >= 3 && std::memcmp(s, "int", 3) != 0)
		score += 5;
	if (n >= 7 && std::memcmp(s, "include", 7) == 0)
		score += 11;
	return score;
}
__attribute__((noinline)) int with_phi(const char* s, unsigned n, int base)
{
	int r = base;
	if (n == 5 && std::memcmp(s, "while", 5) == 0)
		r = base * 2 + 1;
	else if (n == 5)
		r = base - 7;
	return r + static_cast<int>(n);
}
__attribute__((noinline)) int stored_between(char* s, unsigned n)
{
	int r = 0;
	if (n == 3 && std::memcmp(s, "abc", 3) == 0)
		r += 1;
	s[1] = 'b';
	if (n == 3 && std::memcmp(s, "abc", 3) == 0)
		r += 2;
	return r;
}
int main()
{
	const char* probes[] = { "new", "and", "nod", "xor", "delete", "and_eq", "bitor", "compl", "or", "ox", "not_eq", "xor_eq", "xoreq",
		"char", "chat", "int", "inc", "include", "includ", "while", "whale", "a" };
	unsigned sum = 0;
	for (const char* p : probes)
	{
		const unsigned n = static_cast<unsigned>(std::strlen(p));
		const int a = lookup(p, n), b = differs(p, n), c = with_phi(p, n, 10);
		std::printf("%s %d %d %d\n", p, a, b, c);
		sum = sum * 31 + static_cast<unsigned>(a + b + c);
	}
	char buf[4] = { 'a', 'x', 'c', 0 };
	std::printf("stored %d %u\n", stored_between(buf, 3), sum);
	return 0;
}
