// Counted loops leaving from their bodies (host O3 unrolling: each pass's
// exit edge reaches the exit block, whose uses of the loop's values read
// phis of their copies): a table search returning a field, and an early
// return of a running sum and the index.
#include <cstdio>
#include <cstring>
#include <cstdint>
struct Op { const char* text; uint32_t length; uint8_t value; };
static const Op kOps[13] = { { "and", 3, 1 }, { "or", 2, 2 }, { "not", 3, 3 }, { "xor", 3, 4 }, { "bitand", 6, 5 },
	{ "bitor", 5, 6 }, { "compl", 5, 7 }, { "and_eq", 6, 8 }, { "or_eq", 5, 9 }, { "xor_eq", 6, 10 },
	{ "not_eq", 6, 11 }, { "new", 3, 12 }, { "delete", 6, 13 } };
__attribute__((noinline)) uint8_t lookup(const char* s, uint32_t n)
{
	for (const Op& op : kOps)
		if (op.length == n && std::memcmp(op.text, s, n) == 0)
			return op.value;
	return 0;
}
__attribute__((noinline)) int first_big(const int* a, int limit)
{
	int seen = 0;
	for (int k = 0; k < 6; ++k)
	{
		seen += a[k];
		if (a[k] > limit)
			return seen * 100 + k;
	}
	return -seen;
}
int main()
{
	const char* words[] = { "and", "or", "nor", "not_eq", "delete", "new", "bitor", "xor_eq", "x", "compl" };
	unsigned h = 0;
	for (const char* w : words)
		h = h * 31 + lookup(w, static_cast<uint32_t>(std::strlen(w)));
	int a[6] = { 3, 9, 1, 12, 5, 7 };
	for (int limit = 0; limit < 14; ++limit)
		h = h * 7 + static_cast<unsigned>(first_big(a, limit));
	std::printf("%u\n", h);
	return 0;
}
