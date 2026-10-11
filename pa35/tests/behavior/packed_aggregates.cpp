// Small aggregates whose eightbytes move whole (O2+ slot splitting: one
// integer per eightbyte, fields read by shift and truncation, stored by
// mask, shift and or): a Value copied in and out and tested by fields, a
// facts record built field by field and returned or copied out, signed
// and Boolean fields, literals stored into a packed eightbyte, a record
// copied between two locals, and two 32-bit fields kept apart.
#include <cstdint>
#include <cstdio>
#include <vector>
enum class Kind : uint8_t { None, Temp, Slot, Symbol, Integer };
struct Value
{
	Kind kind;
	uint8_t flags;
	uint16_t aux;
	uint32_t index;
};
struct Facts
{
	Kind base = Kind::None;
	uint32_t index = 0;
	int64_t offset = 0;
};
struct Mixed
{
	int8_t small;
	bool flag;
	int16_t medium;
	int32_t large;
};
struct Char
{
	int32_t code;
	uint32_t begin;
	uint32_t end;
};
__attribute__((noinline)) Value resolve(Value v)
{
	if (v.kind == Kind::Temp && v.index % 3 == 0)
		v.index /= 3;
	v.aux = static_cast<uint16_t>(v.aux + 1);
	return v;
}
__attribute__((noinline)) void substitute(Value* ops, unsigned n, const std::vector<Value>& equal)
{
	for (unsigned i = 0; i < n; ++i)
	{
		Value r = resolve(ops[i]);
		if (r.kind == Kind::Temp && r.index < equal.size() && equal[r.index].kind != Kind::None)
			r = equal[r.index];
		if (r.kind == Kind::Integer && i == 0)
			continue;
		ops[i] = r;
	}
}
struct Pass
{
	std::vector<Facts> address;
	__attribute__((noinline)) Facts facts_of(Value v) const
	{
		Facts f;
		if (v.kind == Kind::Slot || v.kind == Kind::Symbol)
		{
			f.base = v.kind;
			f.index = v.index;
			return f;
		}
		if (v.kind == Kind::Temp && v.index < address.size())
		{
			if (address[v.index].base != Kind::None)
				return address[v.index];
			f.base = Kind::Temp;
			f.index = v.index;
		}
		return f;
	}
	void run(const std::vector<Value>& vs, std::vector<Facts>& out) const
	{
		for (unsigned long i = 0; i < vs.size(); ++i)
		{
			Facts f = facts_of(vs[i]);
			Facts g = f;
			if (g.base == Kind::Temp)
				g.offset += 8;
			out[i] = g;
		}
	}
};
__attribute__((noinline)) Mixed build(int x)
{
	Mixed m;
	m.small = static_cast<int8_t>(-x);
	m.flag = x % 2 != 0;
	m.medium = static_cast<int16_t>(x * -300);
	m.large = x * 100000;
	return m;
}
__attribute__((noinline)) long read(const Mixed* ms, int n)
{
	long sum = 0;
	for (int i = 0; i < n; ++i)
	{
		Mixed m = ms[i];
		sum = sum * 7 + m.small + (m.flag ? 1000 : 0) + m.medium + m.large;
	}
	return sum;
}
__attribute__((noinline)) void literals(Mixed* out, int n)
{
	for (int i = 0; i < n; ++i)
	{
		Mixed m = out[i];
		Mixed t = m;
		t.small = -1;
		t.flag = true;
		if (i % 2)
			t.medium = 0x7ff0;
		out[i] = t;
	}
}
__attribute__((noinline)) Char next(const Char* cs, int i, int shift)
{
	Char c = cs[i];
	c.code += shift;
	c.begin += 1;
	return c;
}
int main()
{
	std::vector<Value> equal(12);
	for (unsigned i = 0; i < equal.size(); ++i)
		equal[i] = Value{ i % 4 == 0 ? Kind::None : Kind::Slot, static_cast<uint8_t>(i), static_cast<uint16_t>(i * 300), i * 7 };
	Value ops[6] = { { Kind::Temp, 1, 2, 9 }, { Kind::Integer, 0, 0, 5 }, { Kind::Temp, 3, 65535, 30 },
		{ Kind::Slot, 4, 5, 6 }, { Kind::Temp, 255, 7, 4 }, { Kind::Integer, 9, 9, 9 } };
	substitute(ops, 6, equal);
	unsigned long check = 0;
	for (const Value& v : ops)
		check = check * 131 + static_cast<unsigned>(v.kind) + v.flags * 3u + v.aux * 5u + v.index * 7u;
	Pass p;
	p.address.resize(10);
	p.address[2].base = Kind::Slot;
	p.address[2].index = 44;
	p.address[2].offset = -16;
	std::vector<Value> vs = { { Kind::Slot, 0, 0, 3 }, { Kind::Temp, 0, 0, 2 }, { Kind::Temp, 0, 0, 5 },
		{ Kind::Temp, 0, 0, 50 }, { Kind::Symbol, 0, 0, 1 }, { Kind::Integer, 0, 0, 0 } };
	std::vector<Facts> out(vs.size());
	p.run(vs, out);
	for (const Facts& f : out)
		check = check * 131 + static_cast<unsigned>(f.base) + f.index * 3u + static_cast<unsigned long>(f.offset) * 5u;
	Mixed ms[5];
	for (int i = 0; i < 5; ++i)
		ms[i] = build(i * 37 - 50);
	const long before = read(ms, 5);
	literals(ms, 5);
	Char cs[3] = { { 'a', 0, 1 }, { 0x10ffff, 7, 9 }, { -1, 0xffffffffu, 0 } };
	long chars = 0;
	for (int i = 0; i < 3; ++i)
	{
		const Char c = next(cs, i, i - 1);
		chars = chars * 31 + c.code + static_cast<long>(c.begin) * 3 + c.end;
	}
	std::printf("values %lu mixed %ld %ld chars %ld\n", check, before, read(ms, 5), chars);
	return 0;
}
