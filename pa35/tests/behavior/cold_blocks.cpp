// Blocks that never return placed last (O1+): throws inside loops, a
// shrink-wrapped function whose framed part throws, catch handlers, a
// switch whose default throws, and a cold path that still returns.
#include <cstdio>
#include <stdexcept>
#include <string>
#include <vector>

__attribute__((noinline)) unsigned scan(const std::string& text)
{
	unsigned depth = 0, count = 0;
	for (char c : text)
	{
		if (c == '(')
			++depth;
		else if (c == ')')
		{
			if (depth == 0)
				throw std::runtime_error("unbalanced at " + std::to_string(count));
			--depth;
		}
		else if (c == '!')
			throw std::logic_error("bang");
		++count;
	}
	return depth * 1000 + count;
}

__attribute__((noinline)) int pick(int k, const std::vector<int>& v)
{
	if (k < 0)
		return -1;
	if (static_cast<size_t>(k) >= v.size())
		throw std::out_of_range("pick");
	return v[k] * 3;
}

__attribute__((noinline)) int kind(int x)
{
	switch (x)
	{
	case 0: return 10;
	case 1: return 11;
	case 2: return 13;
	case 3: return 17;
	case 4: return 19;
	case 5: return 23;
	case 6: return 29;
	default: throw std::invalid_argument("kind");
	}
}

__attribute__((noinline)) long rare(long x)
{
	if (x % 97 == 0)
	{
		long s = 0;
		for (long i = 0; i < x % 13; ++i)
			s += i * x;
		return s;
	}
	return x + 1;
}

int main()
{
	const char* texts[] = { "(a(b)c)", "x)y", "((", "ok!", "plain text" };
	unsigned long h = 0;
	for (const char* t : texts)
	{
		try
		{
			h = h * 31 + scan(t);
		}
		catch (const std::runtime_error& e)
		{
			h = h * 7 + static_cast<long>(std::string(e.what()).size());
		}
		catch (const std::logic_error&)
		{
			h += 5;
		}
	}
	std::vector<int> v{ 4, 8, 15, 16, 23, 42 };
	for (int k = -2; k < 9; ++k)
	{
		try
		{
			h = h * 3 + pick(k, v) + kind(k);
		}
		catch (const std::exception& e)
		{
			h = h * 5 + e.what()[0];
		}
	}
	for (long x = 0; x < 500; ++x)
		h = h * 3 + rare(x);
	std::printf("%lu\n", h);
	return 0;
}
