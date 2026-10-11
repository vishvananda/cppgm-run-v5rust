// Switches whose cases return integer literals read a readonly table (host
// O2+, after inlining): a type-size switch with one computed case, signed
// selectors with negative cases, negative and wide results (signed and
// 32-bit entries), 16-, 32- and 64-bit selectors, gaps and values outside
// the cases (the default's literal), and a bool result.
#include <cstdint>
#include <cstdio>

enum class Kind : uint8_t { Void, I1, I8, U8, I16, U16, I32, U32, I64, I128, F32, F64, F80, Ptr, Obj };

struct Shape
{
	uint64_t kind : 8;
	uint64_t align : 8;
	uint64_t bytes : 48;
};

__attribute__((noinline)) uint64_t bytes_of(Shape s)
{
	switch (static_cast<Kind>(s.kind))
	{
	case Kind::Void: return 0;
	case Kind::I1: case Kind::I8: case Kind::U8: return 1;
	case Kind::I16: case Kind::U16: return 2;
	case Kind::I32: case Kind::U32: case Kind::F32: return 4;
	case Kind::I64: case Kind::F64: case Kind::Ptr: return 8;
	case Kind::I128: case Kind::F80: return 16;
	case Kind::Obj: return s.bytes;
	}
	return 0;
}

__attribute__((noinline)) int signed_cases(int x)
{
	switch (x)
	{
	case -3: return -300;
	case -2: return 7;
	case -1: return -1;
	case 0: return 0;
	case 2: return 1000;
	case 4: return -5;
	default: return 42;
	}
}

__attribute__((noinline)) uint32_t wide_results(uint16_t x)
{
	switch (x)
	{
	case 100: return 0xdeadbeefu;
	case 101: return 7;
	case 103: return 0x10000u;
	case 104: return 3;
	default: return 0xffffffffu;
	}
}

__attribute__((noinline)) int64_t long_selector(int64_t x)
{
	switch (x)
	{
	case 10: return -1000000000000ll;
	case 11: return 5;
	case 12: return 6;
	case 15: return 8;
	default: return -7;
	}
}

__attribute__((noinline)) bool small_prime(uint32_t x)
{
	switch (x)
	{
	case 2: case 3: case 5: case 7: case 11: case 13: return true;
	default: return false;
	}
}

int main()
{
	uint64_t h = 0;
	for (unsigned k = 0; k < 20; ++k)
	{
		Shape s = { k, 3, 1000 + k };
		h = h * 31 + bytes_of(s);
	}
	for (int x = -6; x < 8; ++x)
		h = h * 31 + static_cast<uint64_t>(static_cast<int64_t>(signed_cases(x)));
	for (unsigned x = 95; x < 110; ++x)
		h = h * 31 + wide_results(static_cast<uint16_t>(x));
	h = h * 31 + wide_results(0) + wide_results(65535);
	for (int64_t x = 5; x < 20; ++x)
		h = h * 31 + static_cast<uint64_t>(long_selector(x));
	h = h * 31 + static_cast<uint64_t>(long_selector(-12)) + static_cast<uint64_t>(long_selector(INT64_MIN));
	for (uint32_t x = 0; x < 20; ++x)
		h = h * 31 + (small_prime(x) ? 1 : 2);
	h = h * 31 + small_prime(0xffffffffu);
	std::printf("%llu\n", static_cast<unsigned long long>(h));
	return 0;
}
