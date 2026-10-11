// Sparse switches searched by halves (O2+: four or more case runs with
// gaps): signed and unsigned selectors, negative cases, runs at the
// type's bounds, a bit-test window among the runs, the selector in a
// register live past the switch; and a returned struct built in a local
// (named return value) on several paths.
#include <cstdio>
#include <cstdint>
__attribute__((noinline)) int classify(int c)
{
	int r;
	switch (c)
	{
	case -1: r = 1; break;
	case 9: case 11: case 12: case 13: case 32: r = 2; break;
	case 10: r = 3; break;
	case 47: r = 4; break;
	case 100: case 101: case 102: r = 5; break;
	case 2147483647: r = 6; break;
	case -2147483647 - 1: r = 7; break;
	default: r = 0; break;
	}
	return r * 1000 + (c & 0xff);
}
__attribute__((noinline)) unsigned bytes(uint8_t c)
{
	unsigned r;
	switch (c)
	{
	case 0: r = 10; break;
	case 7: r = 11; break;
	case 40: case 41: case 43: case 47: case 50: case 60: r = 12; break;
	case 44: case 45: r = 13; break;
	case 200: r = 14; break;
	case 254: case 255: r = 15; break;
	default: r = 1; break;
	}
	return r + c;
}
__attribute__((noinline)) long wide(uint64_t v)
{
	switch (v)
	{
	case 3: return 1;
	case 1000: return 2;
	case 70000: return 3;
	case 0xffffffffull: return 4;
	case 0x7fffffffull: return 5;
	default: return static_cast<long>(v & 7);
	}
}
struct Found { const void* binding; uint32_t count; bool ambiguous; const void* scope; };
__attribute__((noinline)) void fill(Found& f, int k) { f.binding = k & 1 ? &f : nullptr; f.count = static_cast<uint32_t>(k); f.ambiguous = k % 3 == 0; f.scope = &f; }
__attribute__((noinline)) Found make(int k)
{
	Found f;
	fill(f, k);
	return f;
}
__attribute__((noinline)) Found lookup(int k)
{
	for (int i = 0; i < 3; ++i)
	{
		Found result = make(k + i);
		if (result.binding != nullptr)
		{
			result.count += 100;
			return result;
		}
	}
	return Found{ nullptr, 7, false, nullptr };
}
int main()
{
	long sum = 0;
	for (int c = -300; c < 300; ++c)
		sum += classify(c);
	sum += classify(2147483647) + classify(-2147483647 - 1) + classify(2147483646);
	for (unsigned c = 0; c < 256; ++c)
		sum += bytes(static_cast<uint8_t>(c));
	const uint64_t probes[] = { 0, 3, 4, 999, 1000, 70000, 0x7fffffffull, 0x80000000ull, 0xffffffffull, 0x100000000ull, ~0ull };
	for (uint64_t v : probes)
		sum += wide(v);
	for (int k = 0; k < 20; ++k)
	{
		const Found f = lookup(k * 2);
		sum += f.count + (f.binding != nullptr) + f.ambiguous * 3 + (f.scope != nullptr) * 5;
	}
	std::printf("%ld\n", sum);
	return 0;
}
