// Class objects passed to `...` keep their psABI classes (two SSE
// eightbytes for {double, double}) in calls inlining (O2+) and unrolling
// (O3, straight-line and branching bodies) copy: the selector finds a
// call's stated classes whatever order the copies' entries were added in.
#include <cstdarg>
#include <cstdio>

struct P
{
	double x, y;
};

__attribute__((noinline)) double vsum(int n, ...)
{
	va_list ap;
	va_start(ap, n);
	double s = 0;
	for (int i = 0; i < n; ++i)
	{
		// A P arrives in two SSE registers: two doubles.
		const double x = va_arg(ap, double);
		const double y = va_arg(ap, double);
		s = s * 3 + x - y;
	}
	va_end(ap);
	return s;
}

volatile int vk = 1;

__attribute__((noinline)) double branching(const int* a)
{
	double s = 0;
	for (int i = 0; i < 6; ++i)
	{
		P p = { a[i] * 1.5, 0.25 };
		if (a[i] & 1)
			s += vsum(1, p);
		else
			s -= vsum(1, p);
	}
	P p2 = { a[2] * 0.5, 4.0 };
	return s + vsum(1, p2);
}

__attribute__((noinline)) double straight(P p)
{
	double s = 0;
	for (int i = 0; i < 2; ++i)
		s += vsum(1, p);
	return s;
}

static double callee(const int* a, int i)
{
	P p = { a[i] * 1.5, 2.0 };
	return vsum(1, p);
}

__attribute__((noinline)) double inlined(const int* a)
{
	double s = callee(a, 0);
	P p2 = { a[2] * 0.5, 4.0 };
	s += vsum(1, p2) * 3;
	return s + callee(a, 1);
}

int main()
{
	int a[8];
	for (int i = 0; i < 8; ++i)
		a[i] = i * 7 + vk;
	const P p = { 1.5, 2.75 };
	std::printf("%g %g %g\n", branching(a), straight(p), inlined(a));
	return 0;
}
