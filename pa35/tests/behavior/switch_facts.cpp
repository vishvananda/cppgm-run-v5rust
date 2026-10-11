// Jump threading on switch facts (O2+): a switch's default edge rules out
// its cases, so a later test or switch on the same value (an opcode class
// test, then a test for two of its members) is decided on that path; a
// case edge carries its case values. A reload after a byte store stays
// undecided. Also a switch after a switch on a byte (letters, digits).
#include <cstdio>
struct Inst
{
	unsigned char op;
	unsigned next;
	unsigned char kind;
};
static bool is_eh(unsigned char op)
{
	switch (op)
	{
	case 18: case 19: case 27: case 28: case 29: case 30: case 31: case 32: case 33: case 34: return true;
	default: return false;
	}
}
__attribute__((noinline)) unsigned walk(const Inst* insts, unsigned first, unsigned char* flags, unsigned* handlers)
{
	unsigned n = 0;
	for (unsigned i = first; i != ~0u; i = insts[i].next)
	{
		const Inst& inst = insts[i];
		if (is_eh(inst.op))
			flags[0] |= 8;
		if ((inst.op == 27 || inst.op == 28) && inst.kind == 4)
			handlers[n++] = i;
	}
	return n;
}
__attribute__((noinline)) int classify(unsigned char c)
{
	int r = 0;
	switch (c)
	{
	case 'a': case 'b': case 'c': r = 1; break;
	case '0': case '1': r = 2; break;
	default: r = 3;
	}
	switch (c)
	{
	case 'a': return r * 10;
	case '0': return r * 20;
	case 'z': return r * 30;
	default: return r;
	}
}
int main()
{
	Inst insts[8] = { { 0, 1, 0 }, { 27, 2, 4 }, { 18, 3, 4 }, { 28, 4, 3 }, { 28, 5, 4 }, { 40, 6, 4 }, { 34, 7, 4 },
		{ 5, ~0u, 0 } };
	unsigned char flags[1] = { 0 };
	unsigned handlers[8];
	const unsigned n = walk(insts, 0, flags, handlers);
	std::printf("%u %u", n, flags[0]);
	for (unsigned k = 0; k < n; ++k)
		std::printf(" %u", handlers[k]);
	unsigned long s = 0;
	for (int c = 0; c < 256; ++c)
		s = s * 3 + classify(static_cast<unsigned char>(c));
	std::printf(" %lu\n", s);
	return 0;
}
