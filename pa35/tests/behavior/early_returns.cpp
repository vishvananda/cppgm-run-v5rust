// Returns before the end of a function with pushed preserved registers
// and a frame (O1+ without a frame pointer: each return has its own copy
// of the epilogue and its own unwind rows), with calls after them that
// throw through the function.
#include <cstdio>
__attribute__((noinline)) int thrower(int x)
{
	if (x > 5)
		throw x;
	return x;
}
__attribute__((noinline)) int sink(int x) { return x + 1; }
__attribute__((noinline)) int f(int a, int b, int c, int d)
{
	int s = a * 3, q = b * 5, r = c * 7, w = d * 11;
	if (a == 1)
		return s + q;
	int t = thrower(a) + b;
	if (b == 2)
		return t + s + r;
	int u = thrower(b) * s + sink(q) + sink(r) + sink(w);
	if (c == 3)
		return u - w;
	return u + t + s + q + r + w + thrower(c);
}
int main()
{
	long total = 0;
	for (int a = 0; a < 9; ++a)
		for (int b = 0; b < 9; ++b)
			for (int c = 0; c < 9; ++c)
			{
				try
				{
					total += f(a, b, c, a + b);
				}
				catch (int e)
				{
					total += 1000 * e;
				}
			}
	std::printf("%ld\n", total);
	return 0;
}
