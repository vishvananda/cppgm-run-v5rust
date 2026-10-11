// Object parameters passed by address (a class with a user copy
// constructor) that a function stores whole into its own slot, as an
// inheriting constructor forwarding its by-value parameter does: the store
// copies the object the pointer names, never the pointer itself, at every
// level (O1+ keeps 8-byte objects in registers only when they are values).
#include <cstdint>
#include <cstdio>

int copies, moves, dtors;

template <unsigned N>
struct Box
{
	uint32_t v[N];
	explicit Box(uint32_t x) { for (unsigned i = 0; i < N; ++i) v[i] = x + i * 0x01010101u; }
	Box(const Box& o) { for (unsigned i = 0; i < N; ++i) v[i] = o.v[i]; ++copies; }
	Box(Box&& o) { for (unsigned i = 0; i < N; ++i) v[i] = o.v[i]; ++moves; }
	~Box() { ++dtors; }
	uint32_t sum() const { uint32_t s = 0; for (unsigned i = 0; i < N; ++i) s = s * 31 + v[i]; return s; }
};

struct Base
{
	uint32_t got;
	template <class T>
	Base(T t, int k) : got(t.sum() + static_cast<uint32_t>(k)) {}
	Base(Box<2> b) : got(b.sum() ^ 0x5a5a5a5au) {}
};

struct Derived : Base
{
	using Base::Base;
};

template <unsigned N>
__attribute__((noinline)) uint32_t through(uint32_t x, int k)
{
	Box<N> b(x);
	Derived d(b, k);
	return d.got;
}

__attribute__((noinline)) uint32_t plain(uint32_t x)
{
	Box<2> b(x);
	Derived d(b);
	return d.got;
}

int main(int argc, char**)
{
	const uint32_t x = 0x1234u + static_cast<uint32_t>(argc);
	printf("%08x\n", through<1>(x, 3));
	printf("%08x\n", through<2>(x, 5));
	printf("%08x\n", through<3>(x, 7));
	printf("%08x\n", through<4>(x, 9));
	printf("%08x\n", through<6>(x, 11));
	printf("%08x\n", plain(x));
	printf("copies %d moves %d dtors %d\n", copies, moves, dtors);
	return 0;
}
