// Locals passed by reference (host O2+ read-only call arguments): a local
// whose address only reaches parameters the callee reads through and never
// keeps is passed as a copy and lives in registers; one a callee writes
// through, keeps, returns or compares with another argument keeps its
// object.
#include <cstdio>
#include <vector>

static const int* kept;

__attribute__((noinline)) long weigh(const long& x, const long* table, int n)
{
	long s = 0;
	for (int i = 0; i < n; ++i)
		s += table[i] * x;
	return s;
}

// Read-only through another read-only parameter.
__attribute__((noinline)) long weigh_twice(const long& x, const long* table, int n)
{
	return weigh(x, table, n) + weigh(x, table, n / 2);
}

__attribute__((noinline)) void bump(long& x) { x += 3; }

__attribute__((noinline)) void keep(const int& x) { kept = &x; }

__attribute__((noinline)) int peek() { return *kept; }

__attribute__((noinline)) const int& larger(const int& a, const int& b) { return a < b ? b : a; }

__attribute__((noinline)) bool same(const int& a, const int& b) { return &a == &b; }

__attribute__((noinline)) unsigned depth(const unsigned& n) { return n == 0 ? 0 : 1 + depth(n - 1); }

struct Graph
{
	std::vector<std::vector<unsigned>> adjacent;
	std::vector<unsigned> degree;
	__attribute__((noinline)) void add(unsigned u, unsigned v)
	{
		if (u == v)
			return;
		adjacent[u].push_back(v);
		++degree[u];
		adjacent[v].push_back(u);
		++degree[v];
	}
};

int main()
{
	long table[8];
	for (int i = 0; i < 8; ++i)
		table[i] = i * 7 - 3;
	long acc = 1, total = 0;
	for (int i = 0; i < 20; ++i)
	{
		total += weigh(acc, table, 8);
		acc = acc * 3 % 1000 + i;
		total += weigh_twice(acc, table, 6);
	}
	long b = 4;
	bump(b);
	bump(b);
	int a = 1;
	keep(a);
	a = 5;
	const int seen = peek();
	int x = 7, y = 9;
	const int& r = larger(x, y);
	y = 40;
	const int picked = r;
	const bool both = same(x, x) && !same(x, y);
	unsigned levels = 12;
	const unsigned d = depth(levels);
	Graph g;
	g.adjacent.resize(64);
	g.degree.resize(64);
	for (unsigned u = 0; u < 64; ++u)
		for (unsigned v = u; v < 64; v += 5)
			g.add(u, v);
	unsigned long h = 0;
	for (unsigned u = 0; u < 64; ++u)
	{
		h = h * 31 + g.degree[u];
		for (unsigned v : g.adjacent[u])
			h = h * 7 + v;
	}
	std::printf("%ld %ld %d %d %d %u %lu\n", total, b, seen, picked, both ? 1 : 0, d, h);
	return 0;
}
