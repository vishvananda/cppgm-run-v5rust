// Adds of an immediate whose only use is another add (O1+ over virtual
// registers: lea [x + y + c]): both operands of one add such adds (only one
// folds), 32-bit forms that wrap, subtractions, and adds whose other
// operand is loaded from memory.
#include <cstdint>
#include <cstdio>

long saved_a, saved_b;

__attribute__((noinline)) long both(long v)
{
	const long a = v / 32;
	saved_a = a;
	const long a1 = a + 1;
	const long b = v / 7;
	saved_b = b;
	const long b1 = b + 1;
	return (saved_a + saved_b) + (a1 + b1);
}

__attribute__((noinline)) uint32_t mix(uint32_t h, uint64_t v)
{
	h ^= static_cast<uint32_t>(v) + 0x9e3779b9u + (h << 6) + (h >> 2);
	h ^= static_cast<uint32_t>(v >> 32) + 0x9e3779b9u + (h << 6) + (h >> 2);
	return h;
}

__attribute__((noinline)) uint32_t wrap(uint32_t x, uint32_t y) { return (x + 0xfffffff0u) + y; }

__attribute__((noinline)) long down(long x, const long* p) { return (x - 1000) + *p; }

int main()
{
	long t = both(224) * 3 + both(-224);
	uint32_t h = 17;
	for (uint64_t i = 0; i < 1000; ++i)
		h = mix(h, i * 0x9e3779b97f4a7c15ull);
	const long values[] = { 5, -7, 1 << 20 };
	for (long v : values)
		t = t * 7 + down(v, &values[1]);
	std::printf("%ld %u %u %u\n", t, h, wrap(5, 20), wrap(0xffffffffu, 3));
	return 0;
}
