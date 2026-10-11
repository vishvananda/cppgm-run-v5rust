// Loop headers that load what they test (a null-terminated table walk, a
// linked list, a sentinel scan): jump threading may not take a test the
// header recomputes each pass as decided by the previous pass's branch.
// Also tests after a join on a value the arms did not change.
#include <cstdio>
#include <string>
#include <vector>
static const char* const kPaths[] = { "a", "bb", "ccc", nullptr };
struct Node { int value; Node* next; };
__attribute__((noinline)) void add(std::vector<std::string>& out, bool skip)
{
	if (skip)
		return;
	for (const char* const* p = kPaths; *p != nullptr; ++p)
		out.push_back(*p);
}
__attribute__((noinline)) int walk(const Node* n, int k)
{
	int s = 0;
	while (n != nullptr && n->value != k)
	{
		s += n->value;
		n = n->next;
	}
	return n != nullptr ? s + 1000 : s;
}
__attribute__((noinline)) int scan(const int* p, const int* q)
{
	const int* d = q != nullptr ? q : p;
	int s = 0;
	while (*p != 0)
	{
		s += *p++;
		if (q != nullptr && *d == s)
			break;
	}
	return q != nullptr ? s * 2 : s;
}
int main()
{
	std::vector<std::string> v;
	add(v, false);
	add(v, true);
	add(v, false);
	Node nodes[6];
	for (int i = 0; i < 6; ++i)
		nodes[i] = Node{ i * 3, i + 1 < 6 ? &nodes[i + 1] : nullptr };
	int zs[] = { 4, 5, 6, 7, 0 };
	unsigned n = 0;
	for (const std::string& s : v)
		n = n * 31 + static_cast<unsigned>(s.size());
	std::printf("%zu %u %d %d %d %d\n", v.size(), n, walk(nodes, 9), walk(nodes, 100), scan(zs, nullptr), scan(zs, zs + 1));
	return 0;
}
