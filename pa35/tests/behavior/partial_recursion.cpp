// Partial inlining of recursive callees (pa34 perf 8.3): a caller's call to
// itself gets the callee's fast paths copied in front of the call; a copy
// takes only the copied blocks' temporaries and slots, and the cut keeps
// every returning fast path (a block holding the call is never a fast
// path, even when it returns what the call returns).
#include <cstdio>
#include <cstdint>
#include <climits>
#include <vector>

struct Tree
{
	void build(const std::vector<int32_t>& base)
	{
		size_ = 1;
		while (size_ < base.size())
			size_ <<= 1;
		max_.assign(2 * size_, 0);
		add_.assign(2 * size_, 0);
		for (size_t i = 0; i < base.size(); ++i)
			max_[size_ + i] = base[i];
		for (size_t i = size_; i-- > 1;)
			max_[i] = max_[2 * i] > max_[2 * i + 1] ? max_[2 * i] : max_[2 * i + 1];
	}
	int32_t max(uint32_t first, uint32_t last) const { return query(1, 0, size_ - 1, first, last); }
	void add(uint32_t first, uint32_t last, int32_t delta) { update(1, 0, size_ - 1, first, last, delta); }
	int32_t query(size_t node, size_t lo, size_t hi, size_t first, size_t last) const
	{
		if (last < lo || hi < first)
			return INT32_MIN / 2;
		if (first <= lo && hi <= last)
			return max_[node];
		const size_t mid = (lo + hi) / 2;
		const int32_t a = query(2 * node, lo, mid, first, last);
		const int32_t b = query(2 * node + 1, mid + 1, hi, first, last);
		return (a > b ? a : b) + add_[node];
	}
	void update(size_t node, size_t lo, size_t hi, size_t first, size_t last, int32_t delta)
	{
		if (last < lo || hi < first)
			return;
		if (first <= lo && hi <= last)
		{
			max_[node] += delta;
			add_[node] += delta;
			return;
		}
		const size_t mid = (lo + hi) / 2;
		update(2 * node, lo, mid, first, last, delta);
		update(2 * node + 1, mid + 1, hi, first, last, delta);
		max_[node] = (max_[2 * node] > max_[2 * node + 1] ? max_[2 * node] : max_[2 * node + 1]) + add_[node];
	}
	size_t size_ = 1;
	std::vector<int32_t> max_;
	std::vector<int32_t> add_;
};

// Guard, then a call whose block returns: the call block is not a fast path.
static unsigned long long g_side;
__attribute__((noinline)) static unsigned slow(unsigned x)
{
	g_side += x;
	return x * 2654435761u;
}
static unsigned guarded(unsigned x)
{
	if (x < 8)
		return x + 1;
	return slow(x);
}
// Two guards, two leaves, a local array on the slow path only.
static unsigned digits(unsigned x, unsigned base)
{
	if (base < 2)
		return 0;
	if (x < base)
		return 1;
	unsigned buf[40];
	unsigned n = 0;
	while (x != 0)
	{
		buf[n++] = x % base;
		x /= base;
	}
	unsigned sum = 0;
	for (unsigned i = 0; i < n; ++i)
		sum += buf[i] * (i + 1);
	return n + slow(sum) % 7;
}
// Direct recursion with one guard and a leaf through a local array.
static unsigned collatz(unsigned x, unsigned depth)
{
	if (x <= 1 || depth == 0)
		return depth;
	unsigned seen[4] = {x, x >> 1, x * 3 + 1, depth};
	if ((x & 1) == 0)
		return collatz(seen[1], depth - 1) + seen[0] % 3;
	return collatz(seen[2], depth - 1) + 1;
}
// Mutual recursion: stays out of the partial copies, still correct.
static unsigned even_steps(unsigned x);
static unsigned odd_steps(unsigned x)
{
	if (x == 0)
		return 1;
	return even_steps(x - 1) + 1;
}
static unsigned even_steps(unsigned x)
{
	if (x == 0)
		return 0;
	return odd_steps(x - 1) + 2;
}

int main()
{
	std::vector<int32_t> base(1000);
	for (int i = 0; i < 1000; ++i)
		base[i] = (i * 7919) % 101 - 50;
	Tree t;
	t.build(base);
	long long acc = 0;
	unsigned x = 12345;
	for (int k = 0; k < 20000; ++k)
	{
		x = x * 1103515245u + 12345u;
		unsigned a = (x >> 8) % 1000;
		x = x * 1103515245u + 12345u;
		unsigned b = (x >> 8) % 1000;
		if (a > b)
		{
			unsigned s = a;
			a = b;
			b = s;
		}
		if (k % 3 == 0)
			t.add(a, b, (int)(x % 7) - 3);
		else
			acc += t.max(a, b);
	}
	unsigned long long h = 0;
	for (unsigned i = 0; i < 2000; ++i)
	{
		h = h * 31 + guarded(i % 23);
		h = h * 31 + digits(i * 977, i % 5);
		h = h * 31 + collatz(i, i % 12);
		h = h * 31 + odd_steps(i % 9) + even_steps(i % 7);
	}
	printf("%lld %llu %llu\n", acc, h, g_side);
	return 0;
}
