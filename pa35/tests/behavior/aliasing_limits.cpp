// Type-based aliasing's limits (host O2+): a 32-bit bound and a pointer
// held across 64-bit stores, and what may still alias: a union's members
// written and read through the union, an anonymous union's, an int's bytes
// written through unsigned char, a memcpy from a float into an integer, a
// struct copied whole after a field store, and two pointers of one type.
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <vector>

struct Bits
{
	uint32_t words;
	std::vector<uint64_t> data;
};

__attribute__((noinline)) void merge(Bits& b, const uint64_t* in)
{
	for (uint32_t w = 0; w < b.words; ++w)
		b.data[w] |= in[w];
}

union Pun
{
	uint32_t u;
	float f;
};

__attribute__((noinline)) uint32_t through_union(Pun* p, float x)
{
	p->u = 0x3f800000u;
	p->f = x;
	return p->u;
}

struct Tagged
{
	int kind;
	union
	{
		int i;
		float f;
	};
};

__attribute__((noinline)) int anonymous(Tagged* t, float x)
{
	t->i = 7;
	t->f = x;
	return t->i;
}

__attribute__((noinline)) int bytes(int* p, unsigned char* q)
{
	*p = 0x01020304;
	q[0] = 0xff;
	return *p;
}

__attribute__((noinline)) uint32_t copied(const float* f, uint32_t* u)
{
	*u = 5;
	std::memcpy(u, f, sizeof *u);
	return *u;
}

struct Pair
{
	int a;
	int b;
};

__attribute__((noinline)) long whole(Pair* p, Pair* q)
{
	p->a = 3;
	const Pair c = *p;
	q->b = c.a + 10;
	return static_cast<long>(c.a) * 100 + c.b + p->b;
}

__attribute__((noinline)) long same_type(long* a, long* b)
{
	*a = 1;
	*b = 2;
	return *a;
}

int main()
{
	Bits b;
	b.words = 3;
	b.data.assign(3, 1);
	const uint64_t in[3] = { 2, 4, 8 };
	merge(b, in);
	unsigned long h = b.data[0] * 100 + b.data[1] * 10 + b.data[2];
	Pun pun;
	h = h * 31 + through_union(&pun, 2.0f);
	Tagged t;
	t.kind = 1;
	h = h * 31 + static_cast<unsigned>(anonymous(&t, 1.5f));
	int x = 0;
	h = h * 31 + static_cast<unsigned>(bytes(&x, reinterpret_cast<unsigned char*>(&x)));
	const float f = 0.5f;
	uint32_t u = 0;
	h = h * 31 + copied(&f, &u);
	Pair p = { 1, 2 };
	h = h * 31 + static_cast<unsigned long>(whole(&p, &p));
	long v = 0;
	h = h * 31 + static_cast<unsigned long>(same_type(&v, &v));
	std::printf("%lu\n", h);
	return 0;
}
