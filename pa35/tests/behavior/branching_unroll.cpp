// Counted loops of a few passes with branching bodies (a full unroll at
// -O3 copies them pass by pass): conditional stores, early exits, internal joins with
// phis, values read after the loop, nested fixed loops, and a body that
// leaves from several blocks.
#include <cstdio>
#include <cstdint>

struct Value
{
	uint8_t kind;
	uint8_t pad;
	uint32_t index;
};

struct Inst
{
	Value ops[3];
	uint32_t extra_first;
	uint32_t extra_count;
};

__attribute__((noinline)) void count_uses(const Inst* insts, unsigned n, const Value* extra, unsigned* uses)
{
	for (unsigned i = 0; i < n; ++i)
	{
		for (const Value& v : insts[i].ops)
			if (v.kind == 1)
				++uses[v.index];
		for (uint32_t k = 0; k < insts[i].extra_count; ++k)
			if (extra[insts[i].extra_first + k].kind == 1)
				++uses[extra[insts[i].extra_first + k].index];
	}
}

__attribute__((noinline)) int first_temp(const Inst& inst)
{
	for (int k = 0; k < 3; ++k)
		if (inst.ops[k].kind == 1)
			return k;   // leaves from the body
	return -1;
}

__attribute__((noinline)) unsigned mixed(const int* a)
{
	unsigned acc = 0, odd = 0;
	for (int k = 0; k < 4; ++k)
	{
		unsigned x;
		if (a[k] < 0)
			x = static_cast<unsigned>(-a[k]) * 3u;
		else if (a[k] == 0)
			continue;   // a latch reached from several blocks
		else
			x = static_cast<unsigned>(a[k]);
		acc += x;   // joins the two arms (a phi)
		if (x & 1u)
			++odd;
	}
	return acc * 16u + odd;
}

__attribute__((noinline)) long nested(const long (*m)[3])
{
	long s = 0;
	for (int i = 0; i < 2; ++i)
		for (int j = 0; j < 3; ++j)
			if (m[i][j] > 0)
				s += m[i][j] << (i + j);
			else
				s -= m[i][j];
	return s;
}

__attribute__((noinline)) int find_zero(const int* a, int* where)
{
	int seen = 0;
	for (int k = 0; k < 4; ++k)
	{
		if (a[k] == 0)
		{
			*where = k;
			return seen;   // one exit
		}
		if (a[k] > 100)
			break;   // another, through the exit phi below
		seen += a[k];
	}
	*where = -1;
	return seen;   // seen flows out of the loop through the header's phi
}

int main()
{
	Inst insts[4] = {
		{ { { 1, 0, 0 }, { 1, 0, 2 }, { 0, 0, 7 } }, 0, 2 },
		{ { { 0, 0, 1 }, { 1, 0, 1 }, { 1, 0, 1 } }, 2, 1 },
		{ { { 1, 0, 3 }, { 0, 0, 0 }, { 0, 0, 0 } }, 3, 0 },
		{ { { 0, 0, 0 }, { 0, 0, 0 }, { 1, 0, 0 } }, 0, 3 },
	};
	Value extra[3] = { { 1, 0, 2 }, { 1, 0, 3 }, { 0, 0, 9 } };
	unsigned uses[4] = { 0, 0, 0, 0 };
	count_uses(insts, 4, extra, uses);
	printf("%u %u %u %u\n", uses[0], uses[1], uses[2], uses[3]);
	for (int i = 0; i < 4; ++i)
		printf("%d ", first_temp(insts[i]));
	printf("\n");
	const int a[4] = { -3, 0, 5, 8 };
	const int b[4] = { 2, 4, 6, 8 };
	printf("%u %u\n", mixed(a), mixed(b));
	const long m[2][3] = { { 1, -2, 3 }, { -4, 5, 6 } };
	printf("%ld\n", nested(m));
	int where = 9;
	const int c[4] = { 1, 2, 0, 4 };
	const int d[4] = { 1, 200, 0, 4 };
	const int e[4] = { 1, 2, 3, 4 };
	int r1 = find_zero(c, &where), w1 = where;
	int r2 = find_zero(d, &where), w2 = where;
	int r3 = find_zero(e, &where), w3 = where;
	printf("%d %d %d %d %d %d\n", r1, w1, r2, w2, r3, w3);
	return 0;
}
