// Loads below a join (O1+ memory forwarding, chain_load): a loop body's
// load of a field the preheader set up and the latch reloads after its
// call becomes a phi at the header; a load the body itself precedes with a
// store or a call stays a load; a chain through a diamond's arm, a reload
// after an aliasing store through another pointer, a self-carried value
// (nothing in the loop writes it) and a join whose one predecessor does
// not know the value (no load is ever inserted) all keep their meaning.
#include <cstdio>
#include <cstdint>
#include <vector>
struct Body
{
	std::vector<int> items;
	int* cursor;
	int total;
	int calls;
};
__attribute__((noinline)) void grow(Body& b, int n)
{
	++b.calls;
	if (n % 3 == 0)
		b.items.push_back(n * 2);   // may move the buffer
	b.cursor = b.items.data();
}
__attribute__((noinline)) int walk(Body& b, int n)
{
	int sum = 0;
	for (int i = 0; i < n; ++i)
	{
		int* p = b.items.data();   // reloaded after grow's call in the latch
		sum += p[i % b.items.size()] + b.total;   // b.total: nothing in the loop writes it
		grow(b, i);
	}
	return sum + b.calls;
}
__attribute__((noinline)) int store_first(Body& b, int n)
{
	int sum = 0;
	for (int i = 0; i < n; ++i)
	{
		b.total = i * 3;   // a store before the load: the load reads it
		sum += b.total + b.items[static_cast<size_t>(i) % b.items.size()];
		grow(b, i);
	}
	return sum;
}
static int* alias_target;
__attribute__((noinline)) int through_alias(Body& b, int n)
{
	alias_target = &b.total;
	int sum = 0;
	for (int i = 0; i < n; ++i)
	{
		*alias_target = i;   // the same location through another pointer
		sum += b.total;
		if (i & 1)
			grow(b, i);
	}
	return sum;
}
__attribute__((noinline)) int diamond(Body& b, int n, int flag)
{
	int sum = 0;
	for (int i = 0; i < n; ++i)
	{
		if (flag & i)
			b.total += 1;   // one arm writes
		else
			b.calls += 1;   // the other writes elsewhere
		sum += b.total * 2 + b.calls;
		grow(b, i);
	}
	return sum;
}
__attribute__((noinline)) int unknown_pred(Body& b, int n, int seed)
{
	int sum = seed;
	if (seed > 10)
		grow(b, seed);   // this predecessor of the join knows no value
	for (int i = 0; i < n; ++i)
	{
		sum += b.total + static_cast<int>(b.items.size());
		if (sum & 1)
			grow(b, i);
	}
	return sum;
}
int main()
{
	Body b{ std::vector<int>(4, 5), nullptr, 7, 0 };
	const int w = walk(b, 12);
	const int s = store_first(b, 9);
	const int a = through_alias(b, 8);
	const int d = diamond(b, 10, 5);
	const int u1 = unknown_pred(b, 6, 3), u2 = unknown_pred(b, 6, 30);
	std::printf("walk %d store_first %d alias %d diamond %d unknown %d %d items %zu calls %d total %d\n", w, s, a, d, u1,
		u2, b.items.size(), b.calls, b.total);
	return 0;
}
