// Scaled byte offsets several indexes share made again at each (host
// O2+), so each address takes the scale: arrays of 2-, 4- and 8-byte
// elements indexed by one position and its neighbour, an offset shifted
// rather than multiplied, an offset also used as a value, and negative
// positions.
#include <cstdint>
#include <cstdio>
#include <vector>

struct Columns
{
	std::vector<uint16_t> half;
	std::vector<uint32_t> word;
	std::vector<uint64_t> wide;
};

__attribute__((noinline)) uint64_t sweep(const Columns& c, uint32_t n)
{
	uint64_t s = 0;
	for (uint32_t i = 0; i + 1 < n; ++i)
		s += c.word[i + 1] - c.word[i] + c.half[i] * 3u + c.half[i + 1] + c.wide[i] + (c.wide[i + 1] >> 3);
	return s;
}

__attribute__((noinline)) int64_t pointers(const int32_t* a, const int32_t* b, const int64_t* w, long k)
{
	return a[k] * 5 + b[k] - w[k] + a[k - 1] + b[k - 2] + w[k - 1] + static_cast<int64_t>(k * 4);
}

__attribute__((noinline)) uint64_t shifted(const uint64_t* p, const uint64_t* q, uint64_t k)
{
	const uint64_t off = k << 3;
	const uint64_t* pp = reinterpret_cast<const uint64_t*>(reinterpret_cast<const char*>(p) + off);
	const uint64_t* qq = reinterpret_cast<const uint64_t*>(reinterpret_cast<const char*>(q) + off);
	return *pp * 7 + *qq + off;
}

int main()
{
	Columns c;
	for (uint32_t i = 0; i < 20; ++i)
	{
		c.half.push_back(static_cast<uint16_t>(i * 977));
		c.word.push_back(i * i * 31);
		c.wide.push_back(static_cast<uint64_t>(i) << 37 | i);
	}
	uint64_t h = sweep(c, 20) + sweep(c, 1);
	const int32_t a[6] = { 1, -2, 3, -4, 5, -6 };
	const int32_t b[6] = { 7, 8, -9, 10, 11, 12 };
	const int64_t w[6] = { -100, 200, -300, 400, -500, 600 };
	for (long k = 2; k < 6; ++k)
		h = h * 31 + static_cast<uint64_t>(pointers(a, b, w, k));
	const uint64_t p[4] = { 1, 2, 3, 4 };
	const uint64_t q[4] = { 10, 20, 30, 40 };
	for (uint64_t k = 0; k < 4; ++k)
		h = h * 31 + shifted(p, q, k);
	std::printf("%llu\n", static_cast<unsigned long long>(h));
	return 0;
}
