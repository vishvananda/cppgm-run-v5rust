// Memory updates (O1+): a byte or 16-bit load, operation with an immediate
// and store of one place become `op [m], imm` with the immediate's low bits
// (the stored bits depend on no others), for immediates outside the
// width's range too.
#include <cstdint>
#include <cstdio>

__attribute__((noinline)) void bytes(uint8_t* p, int k)
{
	if (k & 2)
		p[7] = static_cast<uint8_t>(p[7] + -157);
	p[1] = static_cast<uint8_t>(p[1] + 1000);
	p[2] &= ~0x80;
	p[3] &= 0x1f0;
	p[4] ^= 0x1234;
}

__attribute__((noinline)) void words(uint16_t* p)
{
	p[0] ^= 0x12345;
	p[1] = static_cast<uint16_t>(p[1] + 100000);
	p[2] |= 0x30001;
}

int main(int argc, char**)
{
	uint8_t b[8] = { 1, 2, 0xff, 0xff, 7, 9, 11, 13 };
	bytes(b, argc + 1);
	for (int i = 0; i < 8; ++i)
		std::printf("%u ", b[i]);
	uint16_t w[3] = { 1, 65535, 2 };
	words(w);
	std::printf("| %u %u %u\n", w[0], w[1], w[2]);
	return 0;
}
