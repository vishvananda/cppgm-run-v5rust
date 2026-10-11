// By-value aggregates copied into their argument slots and loaded into the
// argument registers: the loads read the copies' sources directly while
// nothing writes memory between (frame copy forwarding); a store or call
// between the copy and the load keeps the slot.
#include <cstdint>
#include <cstdio>

struct Type { uint32_t bits; uint32_t extra; };
struct Value { uint8_t kind; uint8_t a; uint16_t b; uint32_t index; };
struct Wide { uint64_t lo, hi; };
struct Inst { uint8_t op; Type type; Value ops[3]; Wide w; };

__attribute__((noinline)) uint64_t use(Type t, Value v) { return t.bits * 3u + t.extra + v.kind + v.a * 7u + v.b + v.index * 11u; }
__attribute__((noinline)) uint64_t use2(Value a, Value b, Wide w) { return a.index * 5u + b.index + (w.lo ^ w.hi) + a.b - b.a; }
__attribute__((noinline)) void bump(Inst* p) { ++p->type.bits; ++p->ops[1].index; }

__attribute__((noinline)) uint64_t walk(Inst* insts, int n)
{
	uint64_t s = 0;
	for (int i = 0; i < n; ++i)
	{
		s += use(insts[i].type, insts[i].ops[1]);
		s += use2(insts[i].ops[0], insts[i].ops[2], insts[i].w);
		const Type t = insts[i].type;
		bump(&insts[i]);   // the call writes what t was copied from
		s += use(t, insts[i].ops[1]);
		Value v = insts[i].ops[2];
		insts[i].ops[2].index = 99;   // a store between the copy and the call
		s += use(insts[i].type, v);
	}
	return s;
}

int main()
{
	Inst insts[6];
	for (int i = 0; i < 6; ++i)
	{
		insts[i].op = static_cast<uint8_t>(i);
		insts[i].type = Type{ static_cast<uint32_t>(i * 13), static_cast<uint32_t>(i ^ 5) };
		for (int k = 0; k < 3; ++k)
			insts[i].ops[k] = Value{ static_cast<uint8_t>(k + i), static_cast<uint8_t>(i), static_cast<uint16_t>(k * 100 + i), static_cast<uint32_t>(i * 31 + k) };
		insts[i].w = Wide{ static_cast<uint64_t>(i) << 40, static_cast<uint64_t>(i * 977) };
	}
	std::printf("%llu\n", static_cast<unsigned long long>(walk(insts, 6)));
	std::printf("%u %u\n", insts[3].type.bits, insts[5].ops[2].index);
	return 0;
}
