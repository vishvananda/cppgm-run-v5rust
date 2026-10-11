// Memory forwarding through one pointer at constant offsets (O1+): a field
// read again after stores to other fields of the same object is reused,
// and what still reloads: a field a store overlaps through the same
// pointer (a byte of it, a wider word over it), the same field through
// another pointer that may name the same object, and an element at an
// index not known.
#include <cstdint>
#include <cstdio>
#include <cstring>

struct Record
{
	uint32_t a;
	uint32_t b;
	uint64_t c;
	uint32_t items[4];
};

__attribute__((noinline)) uint64_t other_fields(Record* r, uint32_t x)
{
	const uint32_t first = r->a;
	r->b = x;
	r->c = first + 7;
	return first * 10 + r->a + r->b;
}

__attribute__((noinline)) uint64_t overlapping(Record* r, unsigned char byte, uint64_t word)
{
	const uint32_t before = r->a;
	reinterpret_cast<unsigned char*>(r)[1] = byte;
	const uint32_t after_byte = r->a;
	std::memcpy(&r->a, &word, sizeof word);
	return static_cast<uint64_t>(before) * 1000003 + after_byte * 31 + r->a + r->b;
}

__attribute__((noinline)) uint64_t two_pointers(Record* p, Record* q, uint32_t x)
{
	const uint32_t first = p->b;
	q->b = x;
	return static_cast<uint64_t>(first) * 100 + p->b;
}

__attribute__((noinline)) uint64_t unknown_index(Record* r, uint32_t k, uint32_t x)
{
	const uint32_t first = r->items[2];
	r->items[k] = x;
	return static_cast<uint64_t>(first) * 100 + r->items[2];
}

int main()
{
	Record r = { 1, 2, 3, { 4, 5, 6, 7 } };
	uint64_t h = other_fields(&r, 9);
	h = h * 31 + overlapping(&r, 0xab, 0x0000000500000006ull);
	h = h * 31 + two_pointers(&r, &r, 42);
	Record s = r;
	h = h * 31 + two_pointers(&r, &s, 43);
	h = h * 31 + unknown_index(&r, 2, 77) + unknown_index(&r, 1, 78);
	h = h * 31 + r.a + r.b + r.c + r.items[1] + r.items[2];
	std::printf("%llu\n", static_cast<unsigned long long>(h));
	return 0;
}
