// Locals live across setjmp and unmodified until longjmp keep their values
// (C 7.13.2.1p3); spilled ones must not share a slot with values the
// fall-through path defines after setjmp.
#include <csetjmp>
#include <cstdio>

static std::jmp_buf jb;
__attribute__((noinline)) long step(long x) { return x * 3 + 1; }
__attribute__((noinline)) void jump(long x)
{
	if (x & 1)
		std::longjmp(jb, 1);
}

__attribute__((noinline)) long f(long x)
{
	long a = step(x), b = step(a), c = step(b), d = step(c), e = step(d), g = step(e), h = step(g);
	long i = step(h), j = step(i), k = step(j), l = step(k), m = step(l);
	if (setjmp(jb))
		return a + 2 * b + 3 * c + 4 * d + 5 * e + 6 * g + 7 * h + 8 * i + 9 * j + 10 * k + 11 * l + 12 * m;
	// a..m are dead on this path: other spilled values may take their slots.
	long p = step(x + 1), q = step(p), r = step(q), s = step(r), t = step(s), u = step(t), v = step(u);
	long w = step(v), y = step(w), z = step(y), o = step(z), n = step(o);
	jump(x);
	return p + q + r + s + t + u + v + w + y + z + o + n;
}

int main(int argc, char**)
{
	for (long x = 0; x < 4; ++x)
		std::printf("%ld %ld\n", x, f(x + argc));
	return 0;
}
