// Words put together from 32-bit halves stored as the halves (host O2+,
// after inlining): a 12-byte character record assigned from a fast path's
// fields or from a slow path's call (a phi of words), an 8-byte pair built
// from two u32 values, an explicit `a | b << 32` with a 64-bit b (its high
// half truncated), and the stored objects then read whole, by field, and
// after one field is overwritten.
#include <cstdint>
#include <cstdio>
#include <cstring>

struct Char
{
	int32_t code;
	uint32_t begin;
	uint32_t end;
};

__attribute__((noinline)) Char slow(const unsigned char* bytes, uint32_t off)
{
	return Char{ bytes[off] + 1000, off, off + 2 };
}

struct Reader
{
	Char cur;
	const unsigned char* bytes;
	uint32_t size;

	__attribute__((noinline)) void step(uint32_t off)
	{
		if (off < size && bytes[off] < 0x80 && bytes[off] != '\\')
			cur = Char{ bytes[off], off, off + 1 };
		else
			cur = slow(bytes, off);
	}
};

struct Pair
{
	uint32_t lo;
	uint32_t hi;
};

__attribute__((noinline)) void make_pair(Pair* p, uint32_t a, uint32_t b)
{
	*p = Pair{ a * 3, b ^ 0xffffu };
}

__attribute__((noinline)) void packed(uint64_t* out, uint32_t a, uint64_t b)
{
	*out = a | (b << 32);
}

__attribute__((noinline)) uint64_t whole(const void* p)
{
	uint64_t w;
	std::memcpy(&w, p, sizeof w);
	return w;
}

int main()
{
	static const unsigned char text[] = "ab\\c\xc3\xa9z";
	Reader r = { { 0, 0, 0 }, text, sizeof text - 1 };
	uint64_t h = 0;
	for (uint32_t off = 0; off < r.size; ++off)
	{
		r.step(off);
		h = h * 31 + whole(&r.cur) + r.cur.end;
		h = h * 31 + static_cast<uint32_t>(r.cur.code) + r.cur.begin;
		r.cur.begin = 77;
		h = h * 31 + whole(&r.cur);
	}
	Pair p;
	make_pair(&p, 0x12345678u, 0x9abcdef0u);
	h = h * 31 + whole(&p) + p.lo + p.hi;
	uint64_t w = 0;
	packed(&w, 0xdeadbeefu, 0x1122334455667788ull);
	h = h * 31 + w;
	std::printf("%llu\n", static_cast<unsigned long long>(h));
	return 0;
}
