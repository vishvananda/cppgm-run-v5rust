// Early exits before the frame (host O2+ shrink-wrapping: the prologue
// starts the block dominating every use of the frame, the entry's copy of
// an argument into a preserved register moves there, and the exits pop
// nothing), with an exception thrown from the framed part.
#include <cstdio>
#include <vector>
#include <stdexcept>
struct G
{
	std::vector<unsigned> edges;
	std::vector<unsigned char> cls;
	unsigned count = 0;
	__attribute__((noinline)) void add(unsigned u, unsigned v)
	{
		if (u == v)
			return;
		if (u < 4 && v < 4)
			return;
		if (cls[u] != cls[v])
			return;
		if (u > 1000)
			throw std::runtime_error("big");
		edges.push_back(u * 1000 + v);
		++count;
	}
};
int main()
{
	G g;
	g.cls.resize(2000);
	for (unsigned i = 0; i < 2000; ++i)
		g.cls[i] = static_cast<unsigned char>(i % 3);
	unsigned long h = 0;
	for (unsigned u = 0; u < 1100; u += 7)
		for (unsigned v = 0; v < 60; ++v)
		{
			try
			{
				g.add(u, v);
			}
			catch (const std::exception&)
			{
				h += 17;
			}
		}
	for (unsigned e : g.edges)
		h = h * 31 + e;
	std::printf("%lu %u\n", h, g.count);
	return 0;
}
