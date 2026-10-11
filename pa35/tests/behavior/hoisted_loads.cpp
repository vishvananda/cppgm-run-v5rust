// Loop-invariant loads moved to a preheader (O2+, after all inlining): a
// vector's data pointer reloaded around int stores, a member read through
// `this` while other members change, a field the loop itself stores (stays),
// a pointer the loop writes through an int* that may alias it (stays: same
// non-pointer type), a header load guarded by a zero-trip loop over a null
// pointer, a loop with a call (stays), and an identifier-character test
// (bit-test window choice) over mixed-case names.
#include <cstdio>
#include <vector>
struct Table
{
	std::vector<unsigned> next;
	std::vector<unsigned> owner;
	unsigned count;
	long total;
};
__attribute__((noinline)) void map_owners(const Table& t, std::vector<unsigned>& out, const std::vector<unsigned>& heads)
{
	out.assign(t.next.size(), ~0u);
	for (unsigned b = 0; b < heads.size(); ++b)
		for (unsigned i = heads[b]; i != ~0u; i = t.next[i])
			out[i] = b;
}
struct Walker
{
	const int* data;
	unsigned size;
	long sum;
	__attribute__((noinline)) void walk()
	{
		for (unsigned k = 0; k < size; ++k)
			sum += data[k];   // sum stored each pass; data and size invariant
	}
};
__attribute__((noinline)) long self_store(Table& t, unsigned n)
{
	for (unsigned k = 0; k < n; ++k)
		t.count = t.count * 3 + k;   // the loaded field is stored: no hoist
	return t.count;
}
__attribute__((noinline)) long same_type(int* p, int* q, unsigned n)
{
	long s = 0;
	for (unsigned k = 0; k < n; ++k)
	{
		s += *p;   // *q may be *p
		*q = static_cast<int>(k);
	}
	return s;
}
__attribute__((noinline)) long null_guarded(const Table* t, unsigned n)
{
	long s = 0;
	for (unsigned k = 0; k < n; ++k)
		s += t->total;   // n == 0 with t == nullptr: never read
	return s;
}
static long calls = 0;
__attribute__((noinline)) void tick(Table& t) { ++calls; t.total += 1; }
__attribute__((noinline)) long with_call(Table& t, unsigned n)
{
	long s = 0;
	for (unsigned k = 0; k < n; ++k)
	{
		s += t.total;   // the call changes it
		tick(t);
	}
	return s;
}
static bool identifier_char(int c)
{
	return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || c == '_' || (c >= '0' && c <= '9');
}
__attribute__((noinline)) unsigned identifier_lengths(const char* s)
{
	unsigned h = 0, run = 0;
	for (; *s; ++s)
		if (identifier_char(static_cast<unsigned char>(*s)))
			++run;
		else
		{
			h = h * 31 + run;
			run = 0;
		}
	return h * 31 + run;
}
int main()
{
	Table t;
	t.next = { 1, 2, ~0u, 4, ~0u, ~0u };
	t.count = 7;
	t.total = 5;
	std::vector<unsigned> heads = { 0, 3, 5 }, out;
	map_owners(t, out, heads);
	int data[5] = { 3, -1, 4, 1, -5 };
	Walker w = { data, 5, 100 };
	w.walk();
	int a = 9;
	std::printf("%u %u %u %u %u %u %ld\n", out[0], out[1], out[2], out[3], out[4], out[5], w.sum);
	// (Each call sequenced: argument evaluation order is unspecified.)
	const long r1 = self_store(t, 4);
	const long r2 = same_type(&a, &a, 4);
	const long r3 = same_type(&a, data, 3);
	const long r4 = null_guarded(nullptr, 0);
	const long r5 = null_guarded(&t, 3);
	std::printf("%ld %ld %ld %ld %ld\n", r1, r2, r3, r4, r5);
	const long r6 = with_call(t, 4);
	std::printf("%ld %ld %ld\n", r6, calls, t.total);
	std::printf("%u %u\n", identifier_lengths("camelCase snake_case X86_64 p189 a+b (Value) __x"), identifier_lengths(""));
	return 0;
}
