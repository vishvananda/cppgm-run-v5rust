// Loop-invariant arithmetic (O2+, after all inlining): a `size()` bound's
// subtraction and shift computed once before the loop when the end pointer
// is read nowhere else; an invariant shift count widened outside; an
// operand still read after the loop (the arithmetic stays); a zero-trip
// loop over an empty vector.
#include <cstdio>
#include <vector>
struct Graph
{
	std::vector<unsigned> succs, first;
};
__attribute__((noinline)) void count_preds(Graph& g)
{
	for (unsigned k = 0; k < g.succs.size(); ++k)
		++g.first[g.succs[k] + 1];
}
__attribute__((noinline)) unsigned long shifted(const unsigned* v, unsigned n, unsigned char by)
{
	unsigned long s = 0;
	for (unsigned k = 0; k < n; ++k)
		s += static_cast<unsigned long>(v[k]) << (by & 7u);
	return s;
}
__attribute__((noinline)) long after(const long* p, const long* end)
{
	long s = 0;
	for (const long* q = p; q < p + (end - p) / 2; ++q)
		s += *q;
	return s + (end - p);
}
int main()
{
	Graph g;
	g.succs = { 0, 1, 1, 2, 0, 3 };
	g.first.assign(5, 0);
	count_preds(g);
	Graph empty;
	empty.first.assign(1, 7);
	count_preds(empty);
	const unsigned v[5] = { 1, 2, 3, 4, 5 };
	const long w[6] = { 10, 20, 30, 40, 50, 60 };
	std::printf("%u %u %u %u %u %u %lu %ld %ld\n", g.first[0], g.first[1], g.first[2], g.first[3], g.first[4],
		empty.first[0], shifted(v, 5, 11), after(w, w + 6), after(w, w));
	return 0;
}
