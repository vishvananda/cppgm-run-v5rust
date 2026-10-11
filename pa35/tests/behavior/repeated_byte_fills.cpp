// Fills of a value known only at run time (O2+, after inlining): runs of at
// least 128 bytes of whole 16-byte chunks whose value repeats one byte (0,
// ~0, 0x01010101, 0x7f7f7f7f) call memset for the chunks, the rest of the
// run after them; other values (0x01010102, 0x1213), shorter runs and the
// last bytes keep the stores. Element sizes 2, 4 and 8, through
// vector::assign growing and shrinking, and a fill that starts mid-chunk.
#include <cstdio>
#include <vector>
template <typename T> __attribute__((noinline)) void fill(std::vector<T>& v, unsigned n, const T& x) { v.assign(n, x); }
template <typename T> __attribute__((noinline)) void fill_range(T* p, T* end, T x)
{
	for (; p != end; ++p)
		*p = x;
}
template <typename T> unsigned long check(unsigned n, T x)
{
	std::vector<T> v(3, T(7));
	fill(v, n, x);
	unsigned long h = v.size();
	for (unsigned i = 0; i < v.size(); ++i)
		h = h * 31 + static_cast<unsigned long>(v[i]);
	return h;
}
int main()
{
	unsigned long h = 0;
	const unsigned ns[] = { 0, 1, 3, 4, 5, 31, 32, 33, 35, 36, 64, 100, 1000 };
	for (unsigned n : ns)
	{
		h = h * 7 + check<unsigned>(n, 0u) + check<unsigned>(n, ~0u) + check<unsigned>(n, 0x01010101u) +
			check<unsigned>(n, 0x01010102u);
		h = h * 7 + check<unsigned short>(n, 0xffff) + check<unsigned short>(n, 0x1213) + check<unsigned long>(n, 0) +
			check<unsigned long>(n, 0xffffffff00000000ul);
		h = h * 7 + check<int>(n, -1) + check<int>(n, 0x7f7f7f7f);
	}
	unsigned buf[80];
	for (unsigned k = 0; k < 80; ++k)
		buf[k] = k;
	fill_range(buf + 1, buf + 77, 0u);
	fill_range(buf + 3, buf + 40, 0xabababab);
	for (unsigned k = 0; k < 80; ++k)
		h = h * 31 + buf[k];
	std::printf("%lu\n", h);
	return 0;
}
