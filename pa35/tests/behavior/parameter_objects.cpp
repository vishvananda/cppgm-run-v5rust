// Loop loads of pointers hoisted from the objects parameters address (host
// O2+: `this`, a reference, a class passed by address are readable for
// their size even where the loop runs no pass): member vectors read in the
// body of a test-first loop, the same over empty vectors (null pointers,
// never dereferenced), a reference parameter's vector, a class passed by
// value through its address, and a member vector the loop grows (its
// pointers stay loaded on every pass).
#include <cstdint>
#include <cstdio>
#include <vector>

struct Walk
{
	std::vector<uint32_t> next;
	std::vector<uint32_t> weight;
	std::vector<uint32_t> seen;

	__attribute__((noinline)) uint64_t chain(uint32_t from) const
	{
		uint64_t sum = 0;
		for (uint32_t i = from; i != ~0u; i = next[i])
			sum = sum * 3 + weight[i];
		return sum;
	}

	__attribute__((noinline)) uint32_t count(uint32_t n) const
	{
		uint32_t total = 0;
		for (uint32_t k = 0; k < n; ++k)
			total += weight[k] + next[k];
		return total;
	}

	__attribute__((noinline)) void mark(uint32_t from)
	{
		for (uint32_t i = from; i != ~0u; i = next[i])
			seen.push_back(i);
	}
};

__attribute__((noinline)) uint64_t through_reference(const std::vector<uint64_t>& v, uint32_t n)
{
	uint64_t h = 0;
	for (uint32_t k = 0; k < n; ++k)
		h = h * 31 + v[k];
	return h;
}

struct Big
{
	uint64_t pad[6];
	const uint32_t* values;
};

__attribute__((noinline)) uint64_t by_value(Big big, uint32_t n)
{
	uint64_t h = big.pad[0];
	for (uint32_t k = 0; k < n; ++k)
		h = h * 7 + big.values[k];
	return h;
}

int main()
{
	Walk w;
	for (uint32_t i = 0; i < 10; ++i)
	{
		w.next.push_back(i % 4 == 3 || i == 9 ? ~0u : i + 1);
		w.weight.push_back(i * 5 + 1);
	}
	uint64_t h = w.chain(0) + w.chain(4) * 1000 + w.count(10);
	Walk empty;
	h = h * 31 + empty.chain(~0u) + empty.count(0);
	w.mark(8);
	w.mark(0);
	for (uint32_t s : w.seen)
		h = h * 31 + s;
	const std::vector<uint64_t> v = { 5, 6, 7 };
	h = h * 31 + through_reference(v, 3) + through_reference(v, 0);
	const uint32_t values[3] = { 9, 8, 7 };
	Big big = { { 1, 2, 3, 4, 5, 6 }, values };
	h = h * 31 + by_value(big, 3) + by_value(big, 0);
	std::printf("%llu\n", static_cast<unsigned long long>(h));
	return 0;
}
