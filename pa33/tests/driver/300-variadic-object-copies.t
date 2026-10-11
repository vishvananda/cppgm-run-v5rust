// N3485 5.2.2p7, 18.10: a class object passed to `...` is read back by
// va_arg with its value, also when the optimizer copies the call (inlined
// into another function, or in a loop it unrolls) or the va_arg (in a loop
// it unrolls)
#include <cstdarg>
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
		const P p = va_arg(ap, P);
		s = s * 3 + p.x - p.y;
	}
	va_end(ap);
	return s;
}
__attribute__((noinline)) double vthree(int n, ...)
{
	va_list ap;
	va_start(ap, n);
	double s = 0;
	for (int i = 0; i < 3; ++i)
	{
		const P p = va_arg(ap, P);
		s = s * 3 + p.x - p.y;
	}
	va_end(ap);
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
	P p = { a[2] * 0.5, 4.0 };
	s += vsum(1, p) * 3;
	return s + callee(a, 1);
}
__attribute__((noinline)) double unrolled(P p)
{
	double s = 0;
	for (int i = 0; i < 2; ++i)
		s += vsum(1, p);
	return s;
}
int main()
{
	const int a[3] = { 1, 8, 15 };
	const P p = { 1.5, 2.75 }, q = { 2.5, 4.0 }, r = { 8.0, 0.5 };
	if (inlined(a) != 20)
		return 1;
	if (unrolled(p) != -2.5)
		return 2;
	return vthree(3, p, q, r) == -8.25 ? 0 : 3;
}
