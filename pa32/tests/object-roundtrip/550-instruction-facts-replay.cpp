// Instruction facts survive the -O0 LowIR text: typed accesses, signed
// nowrap arithmetic and exact pointer-difference division (12-byte elements
// divide, 8-byte ones shift), with union members and bit-fields untyped.
struct Triple { int a, b, c; };
union Bits { float f; int i; long l; };
struct Flags { int low : 4; int high : 12; short tail; };

__attribute__((noinline)) long span(const Triple *first, const Triple *last)
{
	return last - first;
}

__attribute__((noinline)) long words(const long *first, const long *last)
{
	return last - first;
}

__attribute__((noinline)) int sum(const int *values, int count, Triple *out)
{
	int total = 0;
	for (int i = 0; i < count; ++i)
	{
		total += values[i] * 3;
		out->b = total;
	}
	return total - out->a;
}

__attribute__((noinline)) int pun(Bits *u, float value)
{
	u->f = value;
	return u->i;
}

__attribute__((noinline)) int fields(Flags *flags, int value)
{
	flags->low = value;
	flags->tail = static_cast<short>(value * 2);
	flags->high = value + 1;
	return flags->low + flags->high + flags->tail;
}

int main()
{
	Triple items[5] = {};
	long longs[9] = {};
	int values[4] = {1, 2, 3, 4};
	Bits u;
	Flags flags = {};
	return span(items, items + 5) == 5 && words(longs, longs + 9) == 9 &&
		sum(values, 4, items) == 30 && pun(&u, 1.0f) == 0x3f800000 &&
		fields(&flags, 3) == 13 ? 0 : 1;
}
