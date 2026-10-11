// Early exits before the frame in a function with a cleanup region (host
// O2+ shrink-wrapping: the landing pad belongs to the framed region of
// its call; an argument normalized in its preserved register before the
// prologue is normalized in the argument register instead).
#include <cstdio>
#include <cstdint>
#include <utility>
#include <vector>
#include <stdexcept>
struct A
{
	std::vector<uint64_t> table = std::vector<uint64_t>(8, 0);
	uint64_t n = 0;
	__attribute__((noinline)) void add(unsigned u, unsigned v)
	{
		if (u == v)
			return;
		if (n * 2 > table.size())
		{
			std::vector<uint64_t> old;
			old.swap(table);
			table.assign(old.size() * 2 + 8, 0);
			if (u == 999)
				throw std::runtime_error("growth");
			for (uint64_t k : old)
				if (k != 0)
					table[k % table.size()] = k;
		}
		if (u > v)
			std::swap(u, v);
		table[(static_cast<uint64_t>(u) * 31 + v) % table.size()] = (static_cast<uint64_t>(u) << 32) | v;
		++n;
	}
};
int main()
{
	A a;
	unsigned long h = 0;
	for (unsigned i = 0; i < 300; ++i)
	{
		try
		{
			a.add(i % 13 == 5 ? 999 : i % 7, i % 5);
		}
		catch (const std::exception&)
		{
			h += 3;
		}
	}
	for (uint64_t k : a.table)
		h = h * 31 + k;
	std::printf("%lu %llu\n", h, static_cast<unsigned long long>(a.n));
	return 0;
}
