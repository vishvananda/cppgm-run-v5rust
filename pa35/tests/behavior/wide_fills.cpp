// Wide fills (O2+): a fill loop memset cannot write (an invariant 2-, 4- or
// 8-byte value through a pointer induction tested against an end) runs 16
// bytes a pass over the whole chunks, the original loop finishing the tail.
// Lengths zero to many around the chunk size, every element size, values
// read from memory or parameters, pointer values, the range inside a larger
// array (the neighbours must stay), and the ranges a widened loop's own
// start is derived from.
#include <cstdio>
#include <cstdint>
#include <vector>
template <typename T> __attribute__((noinline)) void fill_range(T* p, T* e, T v)
{
	for (; p != e; ++p)
		*p = v;
}
template <typename T> __attribute__((noinline)) void fill_count(T* p, unsigned long n, const T& v)
{
	T* e = p + n;
	while (p != e)
		*p++ = v;
}
__attribute__((noinline)) void fill_pointers(const char** p, const char** e, const char* v)
{
	for (; p < e; ++p)
		*p = v;
}
__attribute__((noinline)) void fill_signed(int32_t* p, int32_t* e)
{
	for (; p != e; ++p)
		*p = -7;
}
struct Mixer
{
	uint64_t h = 1469598103934665603ull;
	void add(uint64_t v) { h = (h ^ v) * 1099511628211ull; }
};
int main()
{
	Mixer m;
	static const char* const names[3] = { "a", "bb", "ccc" };
	for (unsigned long n = 0; n < 40; ++n)
	{
		uint16_t h[48];
		uint32_t w[48];
		uint64_t q[48];
		int32_t s[48];
		const char* ptrs[48];
		for (int i = 0; i < 48; ++i)
		{
			h[i] = static_cast<uint16_t>(i * 7919);
			w[i] = static_cast<uint32_t>(i * 2654435761u);
			q[i] = static_cast<uint64_t>(i) * 0x9e3779b97f4a7c15ull;
			s[i] = i - 24;
			ptrs[i] = names[i % 3];
		}
		fill_range(h + 3, h + 3 + n, static_cast<uint16_t>(0xbe00 + n));
		fill_range(w + 2, w + 2 + n, w[1]);   // the value from memory
		fill_count(q + 1, n, q[47]);
		fill_pointers(ptrs + 4, ptrs + 4 + n, names[n % 3]);
		fill_signed(s + 5, s + 5 + n);
		for (int i = 0; i < 48; ++i)
		{
			m.add(h[i]);
			m.add(w[i]);
			m.add(q[i]);
			m.add(static_cast<uint32_t>(s[i]));
			m.add(ptrs[i][0]);
		}
	}
	std::vector<uint32_t> v;
	for (unsigned k = 0; k < 40; ++k)
	{
		v.assign((k * 11) % 37, k * 0x01000193u);
		for (uint32_t x : v)
			m.add(x);
		m.add(v.size());
	}
	std::vector<uint64_t> big(1000, 5);
	fill_count(big.data() + 1, 998, static_cast<uint64_t>(0x0102030405060708ull));
	for (uint64_t x : big)
		m.add(x);
	printf("%016llx\n", static_cast<unsigned long long>(m.h));
	return 0;
}
