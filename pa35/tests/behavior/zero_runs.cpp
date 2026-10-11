// Runs of 8-byte zero stores to contiguous words as 16-byte vector stores
// (O1+): runs of four, five, eight and twelve words, words below a pointer
// (negative displacements), words at an index register, a run broken by a
// nonzero word, and pointer fields; the bytes around each run stay.
#include <cstdint>
#include <cstdio>
#include <cstring>

struct Wide
{
	uint64_t guard;
	uint64_t w[12];
	uint64_t tail;
	const char* p[5];
};

__attribute__((noinline)) void clear4(Wide* x)
{
	x->w[0] = 0;
	x->w[1] = 0;
	x->w[2] = 0;
	x->w[3] = 0;
}

__attribute__((noinline)) void clear5_broken(Wide* x)
{
	x->w[2] = 0;
	x->w[3] = 0;
	x->w[4] = 0;
	x->w[5] = 7;
	x->w[6] = 0;
	x->w[7] = 0;
	x->w[8] = 0;
	x->w[9] = 0;
	x->w[10] = 0;
}

__attribute__((noinline)) void clear12(Wide* x)
{
	for (unsigned k = 0; k < 12; ++k)
		x->w[k] = 0;
	x->p[0] = nullptr;
	x->p[1] = nullptr;
	x->p[2] = nullptr;
	x->p[3] = nullptr;
	x->p[4] = nullptr;
}

__attribute__((noinline)) void clear_below(uint64_t* end)
{
	end[-1] = 0;
	end[-2] = 0;
	end[-3] = 0;
	end[-4] = 0;
	end[-5] = 0;
}

__attribute__((noinline)) void clear_at(uint64_t* base, long i)
{
	base[i] = 0;
	base[i + 1] = 0;
	base[i + 2] = 0;
	base[i + 3] = 0;
	base[i + 4] = 0;
	base[i + 5] = 0;
	base[i + 6] = 0;
	base[i + 7] = 0;
}

__attribute__((noinline)) uint64_t digest(const Wide& x)
{
	uint64_t h = x.guard;
	for (uint64_t v : x.w)
		h = h * 31 + v;
	h = h * 31 + x.tail;
	for (const char* s : x.p)
		h = h * 31 + (s == nullptr ? 1 : 2);
	return h;
}

int main()
{
	static const char text[] = "x";
	Wide x;
	auto fill = [&]() {
		x.guard = 0x1111;
		for (unsigned k = 0; k < 12; ++k)
			x.w[k] = k * 0x0101010101010101ull + 1;
		x.tail = 0x2222;
		for (auto& s : x.p)
			s = text;
	};
	uint64_t h = 0;
	fill();
	clear4(&x);
	h = h * 31 + digest(x);
	fill();
	clear5_broken(&x);
	h = h * 31 + digest(x);
	fill();
	clear12(&x);
	h = h * 31 + digest(x);
	fill();
	clear_below(&x.w[7]);
	h = h * 31 + digest(x);
	fill();
	clear_at(x.w, 3);
	h = h * 31 + digest(x);
	std::printf("%llu\n", static_cast<unsigned long long>(h));
	return 0;
}
