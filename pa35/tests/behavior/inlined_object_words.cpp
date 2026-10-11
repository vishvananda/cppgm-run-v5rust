// An object parameter converted to its word (host objects: object slots
// as i64 words) inlined at a call passing a slot: the slot's object is
// loaded for the conversion (once miscompiled at -O3: the copy took the
// slot's address, and the self-built compiler unrolled loops wrongly).
#include <cstdio>
#include <cstdint>
#include <vector>
enum class Kind : uint8_t { None, Temp, Slot, Integer };
struct Value
{
	Kind kind;
	uint8_t extra;
	uint32_t index;
	bool is(Kind k) const { return kind == k; }
	static Value temp(uint32_t i) { Value v; v.kind = Kind::Temp; v.extra = 0; v.index = i; return v; }
	static Value none() { Value v; v.kind = Kind::None; v.extra = 0; v.index = 0; return v; }
};
struct Inst { uint32_t op; uint32_t result; Value ops[3]; uint32_t first, count; };
__attribute__((noinline)) unsigned run(std::vector<Inst>& body, std::vector<Value>& operands, unsigned temps)
{
	std::vector<Value> map(temps, Value::none());
	auto mapped = [&](Value v) {
		return v.is(Kind::Temp) && v.index < map.size() && !map[v.index].is(Kind::None) ? map[v.index] : v;
	};
	std::vector<Inst> out;
	for (int pass = 0; pass < 3; ++pass)
		for (size_t i = 0; i < body.size(); ++i)
		{
			Inst inst = body[i];
			for (Value& v : inst.ops)
				v = mapped(v);
			std::vector<Value> extra;
			for (uint32_t k = 0; k < inst.count; ++k)
				extra.push_back(mapped(operands[inst.first + k]));
			inst.first = static_cast<uint32_t>(operands.size());
			operands.insert(operands.end(), extra.begin(), extra.end());
			if (inst.result != ~0u)
			{
				const uint32_t fresh = temps++;
				map.resize(temps, Value::none());
				map[inst.result] = Value::temp(fresh);
				inst.result = fresh;
			}
			out.push_back(inst);
		}
	unsigned h = 0;
	for (const Inst& inst : out)
	{
		for (const Value& v : inst.ops)
			h = h * 31 + static_cast<unsigned>(v.kind) * 1000 + v.extra * 100 + v.index;
		for (uint32_t k = 0; k < inst.count; ++k)
			h = h * 7 + operands[inst.first + k].index + static_cast<unsigned>(operands[inst.first + k].kind);
		h += inst.result;
	}
	return h;
}
int main()
{
	std::vector<Inst> body;
	std::vector<Value> operands;
	for (uint32_t i = 0; i < 8; ++i)
	{
		Inst inst;
		inst.op = i;
		inst.result = i % 3 == 2 ? ~0u : i;
		for (int k = 0; k < 3; ++k)
		{
			inst.ops[k] = Value::temp((i + k) % 8);
			inst.ops[k].extra = static_cast<uint8_t>(k + i);
			if (k == 2)
				inst.ops[k].kind = Kind::Integer;
		}
		inst.first = static_cast<uint32_t>(operands.size());
		inst.count = i % 2;
		for (uint32_t k = 0; k < inst.count; ++k)
			operands.push_back(Value::temp((i * 3 + k) % 8));
		body.push_back(inst);
	}
	std::printf("%u\n", run(body, operands, 8));
	return 0;
}
