// Early exits before the frame whose tests reuse an argument register
// (host O2+ shrink-wrapping): the argument a preserved register keeps for
// the framed part waits in another caller-saved register until the
// prologue, which copies it from there.
#include <cstdio>
#include <vector>

__attribute__((noinline)) unsigned bump(unsigned c) { return c + 1; }

struct Edges
{
	std::vector<unsigned char> cls;
	std::vector<std::vector<unsigned>> lists;
	std::vector<unsigned> table;
	unsigned count = 0;
	__attribute__((noinline)) void add(unsigned u, unsigned v)
	{
		if (u == v || (u < 8 && v < 8))
			return;
		const unsigned cu = u < 8 ? (u >= 4) : cls[u - 8];
		const unsigned cv = v < 8 ? (v >= 4) : cls[v - 8];
		if (cu != cv)
			return;
		count = bump(count);
		table.push_back(u < v ? u * 4096 + v : v * 4096 + u);
		if (u >= 8)
			lists[u].push_back(v);
		if (v >= 8)
			lists[v].push_back(u);
	}
};

int main()
{
	Edges e;
	e.cls.resize(200);
	e.lists.resize(208);
	for (unsigned i = 0; i < 200; ++i)
		e.cls[i] = static_cast<unsigned char>((i * 7) % 2);
	for (unsigned u = 0; u < 120; ++u)
		for (unsigned v = 0; v < 40; v += 3)
			e.add(u, (u * v) % 150);
	unsigned long h = e.count;
	for (const std::vector<unsigned>& l : e.lists)
		for (unsigned v : l)
			h = h * 31 + v;
	std::printf("%u %lu\n", e.count, h);
	return 0;
}
