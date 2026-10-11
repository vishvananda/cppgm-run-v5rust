// Shrink-wrapped fast paths holding bulk copies and zeroing (frameless at
// -O1+: xmm15 and r10/r11 chunks), whose slow paths call, keep values in
// callee-saved registers and throw; copies of 8 to 128 bytes, a copy whose
// addresses name many registers, and bit scans on the way.
#include <cstdio>
#include <cstring>
#include <stdexcept>
#include <vector>
struct Tok { long a, b, c, d; };
struct Big { long w[16]; };
struct Odd { char c[13]; };
static long acc = 0;
__attribute__((noinline)) void slow(long x) { acc += x; if (x == 977) throw std::runtime_error("slow"); }
__attribute__((noinline)) void push(std::vector<Tok>& v, const Tok& t) { v.push_back(t); }
__attribute__((noinline)) long copy_big(Big* dst, const Big* src, long n) {
	if (n > 1000) { slow(n); return -1; }
	*dst = *src;
	return dst->w[n & 15];
}
__attribute__((noinline)) long copy_odd(Odd* dst, const Odd* src, long k) {
	if (k < 0) { slow(-k); return 0; }
	*dst = *src;
	return dst->c[k % 13];
}
__attribute__((noinline)) long zero_big(Big* b, long k) {
	if (k == 3) { slow(k); return 7; }
	std::memset(b, 0, sizeof *b);
	b->w[k & 15] = k;
	return b->w[(k + 1) & 15];
}
__attribute__((noinline)) long indexed(Tok* base, long i, long j, const Tok* src, long n) {
	if (n > 500) { slow(n); return 1; }
	base[i] = src[j];
	base[j + 1] = src[i];
	return base[i].b + base[j + 1].c + __builtin_ctzl(n | 64) + (63 - __builtin_clzl(n | 1));
}
int main() {
	std::vector<Tok> v;
	Big a = {}, b = {};
	Odd o = {}, p = {};
	Tok ts[8] = {};
	long s = 0;
	for (long i = 0; i < 2000; ++i) {
		Tok t = { i, i * 2, i * 3, i ^ 5 };
		push(v, t);
		for (int k = 0; k < 16; ++k) a.w[k] = i + k;
		for (int k = 0; k < 13; ++k) p.c[k] = static_cast<char>(i + k);
		try {
			s += copy_big(&b, &a, i);
			s += copy_odd(&o, &p, i % 40 - 20);
			s += zero_big(&b, i % 9);
			ts[i & 7] = t;
			s += indexed(ts, i & 3, (i >> 2) & 3, ts + 4, i % 600);
		} catch (const std::exception& e) {
			s += 1000;
		}
	}
	for (const Tok& t : v) s += t.a ^ t.d;
	std::printf("%ld %ld %zu\n", s, acc, v.size());
	return 0;
}
