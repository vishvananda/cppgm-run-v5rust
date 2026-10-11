// Compare chains extending a switch (host O2+): a default block that tests
// the selector against one more literal joins the switch only when the
// switch reaches it through its default alone; with a case also leading
// there (`case 3: default:`), that case still needs the block.
#include <cstdio>

volatile int sink;

__attribute__((noinline)) int one_case(int x, int y)
{
	int r;
	switch (x)
	{
	case 1: r = y + 1; break;
	case 2: r = y * 3; break;
	case 3:
	default:
		if (x == 7)
		{
			r = y - 100;
			sink = 1;
			break;
		}
		r = y;
	}
	return r;
}

__attribute__((noinline)) int several(int x, int y)
{
	int r;
	switch (x)
	{
	case 1: r = y + 1; break;
	case 2: r = y * 3; break;
	case 3:
	case 4:
	case 5:
	default:
		if (x == 7)
		{
			r = y - 100;
			sink = 1;
			break;
		}
		r = y;
	}
	return r;
}

__attribute__((noinline)) int default_only(int x, int y)
{
	int r;
	switch (x)
	{
	case 1: r = y + 1; break;
	case 2: r = y * 3; break;
	case 3: r = y - 3; break;
	default:
		if (x == 7)
		{
			r = y - 100;
			sink = 1;
			break;
		}
		r = y;
	}
	return r;
}

int main(int argc, char**)
{
	for (int x = -1; x < 10; ++x)
		std::printf("%d: %d %d %d\n", x, one_case(x, argc * 1000), several(x, argc * 1000), default_only(x, argc * 1000));
	return 0;
}
