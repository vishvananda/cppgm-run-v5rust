// Type-based aliasing in memory forwarding (host O2+: a pointer access and
// a 16/32-bit or floating access do not alias) separates accesses through
// different bases only: through one union, at overlapping offsets, a write
// of one member ends what was known of another (GCC's union punning).
#include <cstdint>
#include <cstdio>

union U
{
	void* p;
	uint32_t w[2];
	uint16_t h[4];
};

__attribute__((noinline)) uint32_t pointer_then_word(U* u, void* q, uint32_t x)
{
	u->w[0] = x;
	u->p = q;
	return u->w[0];
}

__attribute__((noinline)) void* word_then_pointer(U* u, void* q, uint32_t x)
{
	u->p = q;
	u->w[0] = x;
	return u->p;
}

__attribute__((noinline)) void* half_then_pointer(U* u, void* q, uint16_t x)
{
	u->p = q;
	u->h[1] = x;
	return u->p;
}

__attribute__((noinline)) uintptr_t local(void* a, uint32_t lo)
{
	U x;
	x.p = a;
	const uint32_t high = x.w[1];
	x.w[0] = lo;
	return reinterpret_cast<uintptr_t>(x.p) + high;
}

volatile uintptr_t vq = 0x1122334455667788ull;

int main(int argc, char**)
{
	U u;
	void* q = reinterpret_cast<void*>(vq);
	const uint32_t x = 0xdeadbeefu + static_cast<uint32_t>(argc);
	std::printf("%x\n", pointer_then_word(&u, q, x));
	std::printf("%lx\n", static_cast<unsigned long>(reinterpret_cast<uintptr_t>(word_then_pointer(&u, q, x))));
	std::printf("%lx\n", static_cast<unsigned long>(reinterpret_cast<uintptr_t>(half_then_pointer(&u, q, 0xabcd))));
	std::printf("%lx\n", static_cast<unsigned long>(local(q, 0xabcd0000u + static_cast<uint32_t>(argc))));
	return 0;
}
