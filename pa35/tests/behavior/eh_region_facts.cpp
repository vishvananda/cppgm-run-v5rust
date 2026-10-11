// Memory facts across EH region markers (O1+, memory forwarding): a
// function with a local vector (a cleanup region) pushes pairs built from
// variables, so an `eh_try` sits between the pair's stores and its copy;
// a call inside the region throws, and the handler sees memory as the
// throw left it (a slot written before the region, one written inside).
#include <cstdio>
#include <stdexcept>
#include <utility>
#include <vector>
static int calls = 0;
__attribute__((noinline)) unsigned maybe_throw(unsigned k)
{
	++calls;
	if (k == 5)
		throw std::runtime_error("five");
	return k * 2;
}
__attribute__((noinline)) unsigned long walk(unsigned n, unsigned* seen)
{
	std::vector<std::pair<unsigned, unsigned>> stack;
	unsigned long h = 0;
	for (unsigned b = 0; b < n; ++b)
	{
		stack.push_back(std::make_pair(b, 0u));
		seen[0] = b;
		const unsigned v = maybe_throw(b);
		seen[1] = v;
		h = h * 31 + v + stack.back().first;
	}
	return h + stack.size();
}
int main()
{
	unsigned seen[2] = { 0, 0 };
	unsigned long h = walk(4, seen);
	std::printf("%lu %u %u\n", h, seen[0], seen[1]);
	try
	{
		h = walk(9, seen);
	}
	catch (const std::exception& e)
	{
		std::printf("caught %s %u %u %d\n", e.what(), seen[0], seen[1], calls);
	}
	return 0;
}
