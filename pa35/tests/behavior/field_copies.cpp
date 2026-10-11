// Copies of small temporaries whose fields were just stored (O1+, memory
// forwarding): push_back(make_pair(b, 0u)) stores b and 0 at the vector's
// end instead of reloading the 8-byte temporary right after its two 4-byte
// stores (a store-forwarding stall); a pointer-and-two-ints record; a
// temporary with a field the copy cannot know (copied whole); growth
// through the slow path (the temporary's address goes to the call).
#include <cstdio>
#include <utility>
#include <vector>
struct Record
{
	const int* p;
	unsigned a, b;
};
__attribute__((noinline)) void push_pairs(std::vector<std::pair<unsigned, unsigned>>& stack, unsigned n)
{
	for (unsigned b = 0; b < n; ++b)
		stack.push_back(std::make_pair(b * 3, 0u));
}
__attribute__((noinline)) void push_records(std::vector<Record>& out, const int* base, unsigned n)
{
	for (unsigned k = 0; k < n; ++k)
	{
		Record r;
		r.p = base + k;
		r.a = k;
		r.b = k ^ 5;
		out.push_back(r);
	}
}
__attribute__((noinline)) void push_partial(std::vector<std::pair<unsigned, unsigned>>& out, const unsigned* src, unsigned n)
{
	for (unsigned k = 0; k < n; ++k)
	{
		std::pair<unsigned, unsigned> p;
		p.first = k;
		p.second = src[k];
		src[k] == 7 ? out.push_back(std::make_pair(p.second, p.first)) : out.push_back(p);
	}
}
int main()
{
	std::vector<std::pair<unsigned, unsigned>> stack, partial;
	push_pairs(stack, 9);
	const int data[6] = { 4, 8, 15, 16, 23, 42 };
	std::vector<Record> records;
	push_records(records, data, 6);
	const unsigned src[5] = { 3, 7, 1, 7, 9 };
	push_partial(partial, src, 5);
	unsigned long h = 0;
	for (const auto& p : stack)
		h = h * 31 + p.first * 7 + p.second;
	for (const Record& r : records)
		h = h * 31 + static_cast<unsigned long>(*r.p) + r.a * 3 + r.b;
	for (const auto& p : partial)
		h = h * 31 + p.first * 5 + p.second;
	std::printf("%zu %zu %zu %lu\n", stack.size(), records.size(), partial.size(), h);
	return 0;
}
