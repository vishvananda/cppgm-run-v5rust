// Counted fills (O2+): a count down to zero beside a pointer tests the
// pointer against its end, and the value a fill stores, read through a
// reference every pass, is read once (a store at its own address writes
// what it holds). Lengths zero to many, the value inside the range filled,
// element sizes 1 to 8, vector::assign growing and shrinking.
#include <cstdio>
#include <cstdint>
#include <memory>
#include <vector>
template <typename T> __attribute__((noinline)) T* fill_n_ref(T* p, unsigned long n, const T& v)
{
	for (; n > 0; --n, ++p)
		*p = v;
	return p;
}
template <typename T> __attribute__((noinline)) void uninit(T* p, unsigned long n, const T& v)
{
	std::uninitialized_fill_n(p, n, v);
}
int main()
{
	uint64_t sum = 0;
	uint32_t a[64];
	for (int i = 0; i < 64; ++i)
		a[i] = static_cast<uint32_t>(i * 2654435761u);
	for (unsigned long n = 0; n < 20; n += 3)
	{
		uint32_t* end = fill_n_ref(a + 5, n, a[6]);   // the value inside the range
		sum = sum * 31 + static_cast<uint64_t>(end - a);
	}
	for (int i = 0; i < 64; ++i)
		sum = sum * 31 + a[i];
	uint8_t b[40] = {};
	fill_n_ref(b + 1, 30, b[0]);
	uint64_t w[10];
	uninit(w, 10, static_cast<uint64_t>(0x0123456789abcdefull));
	uint16_t h[9];
	uninit(h, 9, static_cast<uint16_t>(0xbeef));
	uninit(h, 0, static_cast<uint16_t>(1));
	for (int i = 0; i < 40; ++i)
		sum = sum * 31 + b[i];
	for (int i = 0; i < 10; ++i)
		sum = sum * 31 + w[i];
	for (int i = 0; i < 9; ++i)
		sum = sum * 31 + h[i];
	std::vector<unsigned> v;
	for (unsigned k = 0; k < 12; ++k)
	{
		v.assign((k * 7) % 23, k * 0x01000193u);
		for (unsigned x : v)
			sum = sum * 31 + x;
		sum += v.size();
	}
	std::vector<int> iv(5, -3);
	iv.assign(17, iv[2]);   // the value is an element being replaced
	for (int x : iv)
		sum = sum * 31 + static_cast<unsigned>(x);
	std::printf("%llu\n", static_cast<unsigned long long>(sum));
	return 0;
}
