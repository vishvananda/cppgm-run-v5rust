// Sibling calls (tail jumps at -O2+): direct, indirect and virtual
// targets, integer, floating and pointer results, values in callee-saved
// registers across the frame, a callee that throws through tail-jumped
// frames, and a frame object whose address escapes (no tail jump there).
#include <cstdio>
#include <stdexcept>
static long acc = 0;
__attribute__((noinline)) long leaf(long a, long b) { acc += a; return a * 3 + b; }
__attribute__((noinline)) double fleaf(double x, long n) { return x * n + 0.5; }
__attribute__((noinline)) const char* pleaf(int k) { return k & 1 ? "odd" : "even"; }
__attribute__((noinline)) long thrower(long x) { if (x % 7 == 3) throw std::runtime_error("seven"); return x + 1; }
__attribute__((noinline)) long direct(long a, long b) { if (a < 0) return 0; return leaf(a + 1, b ^ 5); }
__attribute__((noinline)) double fdirect(double x, long n) { return fleaf(x * 2, n + 1); }
__attribute__((noinline)) const char* pdirect(int k) { return pleaf(k + 1); }
typedef long (*Fn)(long, long);
__attribute__((noinline)) long indirect(Fn f, long a, long b) { return f(a - 1, b + 2); }
struct Base { virtual long go(long x) = 0; virtual ~Base() {} };
struct D1 : Base { long go(long x) override { return x * 5; } };
struct D2 : Base { long go(long x) override { return x - 9; } };
__attribute__((noinline)) long virt(Base* b, long x) { return b->go(x + 3); }
__attribute__((noinline)) long many(long a, long b, long c, long d, long e, long f) {
	long s = a * b + c * d + e * f;
	acc ^= s;
	return leaf(s, a + f);
}
__attribute__((noinline)) long throws_through(long x) { return thrower(x * 2 + 1); }
__attribute__((noinline)) long reads_local(long x) { long local = x * 7; acc += local; return leaf(local, (long)&local % 3 == 9); }
__attribute__((noinline)) long escapes(long x) { long box[4] = { x, x + 1, x + 2, x + 3 }; return indirect([](long p, long q) { return p + q; }, box[x & 3], (long)(box + 1) == 0); }
int main() {
	D1 d1; D2 d2; Base* bs[2] = { &d1, &d2 };
	long sum = 0; double fs = 0; int caught = 0, odd = 0;
	for (long i = -3; i < 200; ++i) {
		sum += direct(i, i * 3) + indirect(i & 1 ? leaf : direct, i, i + 4) + virt(bs[i & 1], i) + many(i, i + 1, i + 2, i + 3, i + 4, i + 5);
		fs += fdirect(i * 0.25, i);
		odd += pdirect((int)i)[0] == 'o';
		sum += reads_local(i) + escapes(i);
		try { sum += throws_through(i); } catch (const std::exception&) { ++caught; }
	}
	std::printf("%ld %.2f %d %d %ld\n", sum, fs, caught, odd, acc);
}
