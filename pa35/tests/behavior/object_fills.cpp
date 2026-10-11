// Fill loops copying a 1-, 2- or 8-byte object from one place (host objects:
// a memset when its bytes are alike, checked before the loop): range and
// counted forms, a by-value object, and vector::assign of a small struct.
#include <cstdio>
#include <cstdint>
#include <vector>
#include <algorithm>
struct V { uint8_t kind; uint8_t pad; uint32_t index; };
struct B { uint8_t a; };
struct W { uint16_t a; };
__attribute__((noinline)) void fill_v(V* p, V* e, const V& v) { for (; p != e; ++p) *p = v; }
__attribute__((noinline)) void fill_vn(V* p, unsigned long n, const V& v) { for (; n--; ++p) *p = v; }
__attribute__((noinline)) void fill_b(B* p, B* e, const B& v) { for (; p != e; ++p) *p = v; }
__attribute__((noinline)) void fill_w(W* p, int n, W v) { for (int i = 0; i < n; ++i) p[i] = v; }
int main()
{
	V a[30];
	B b[30];
	W w[30];
	unsigned long h = 0;
	for (int k = 0; k < 4; ++k)
	{
		for (auto& x : a) x = V{ 9, 9, 9 };
		V v = { static_cast<uint8_t>(k == 1 ? 0 : k), 0, static_cast<uint32_t>(k == 1 ? 0 : k * 0x01010101u) };
		fill_v(a + 2, a + 20 - k, v);
		fill_vn(a + 21, static_cast<unsigned long>(k), k == 3 ? V{ 0xff, 0xff, 0xffffffffu } : v);
		for (auto& x : a) h = h * 31 + x.kind + x.pad + x.index;
		for (auto& x : b) x.a = 1;
		fill_b(b + k, b + 25, B{ static_cast<uint8_t>(k * 50) });
		for (auto& x : b) h = h * 31 + x.a;
		for (auto& x : w) x.a = 3;
		fill_w(w, 7 * k, W{ static_cast<uint16_t>(k == 2 ? 0x4141 : k * 1000) });
		for (auto& x : w) h = h * 31 + x.a;
	}
	std::vector<V> vec(5, V{ 1, 0, 2 });
	vec.assign(40, V{ 0, 0, 0 });
	for (auto& x : vec) h = h * 7 + x.index + x.kind;
	vec.assign(12, V{ 3, 0, 5 });
	for (auto& x : vec) h = h * 7 + x.index + x.kind;
	std::printf("%lu\n", h);
	return 0;
}
