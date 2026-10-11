// Loads after a diamond reuse what its dominating block read, less what the
// arms' writes may reach (O1+; host O2+ also by C++ type-based aliasing):
// an instruction walk rereading a pointer field after arms that bump 32-bit
// counters, a field an arm stores through a pointer of the same type or
// writes in a call (reloaded), a global an arm assigns, an arm copying a
// whole object and its sibling storing one field, and a byte store that may
// alias anything.
#include <cstdint>
#include <cstdio>
#include <vector>

struct Inst
{
	uint8_t op;
	uint32_t next;
	uint32_t a;
	uint32_t b;
};

struct Body
{
	std::vector<uint32_t> first;
	std::vector<Inst> insts;
};

struct Counter
{
	Body& body_;
	std::vector<uint32_t> users_;
	explicit Counter(Body& b) : body_(b) {}
	__attribute__((noinline)) void run()
	{
		for (uint32_t b = 0; b < body_.first.size(); ++b)
			for (uint32_t i = body_.first[b]; i != ~0u; i = body_.insts[i].next)
			{
				const Inst& inst = body_.insts[i];
				if (inst.op == 1)
					++users_[inst.a];
				if (inst.op == 2)
					++users_[inst.b];
			}
	}
};

struct Node
{
	Node* next;
	long value;
};

__attribute__((noinline)) void relink(Node** where, Node* to) { *where = to; }

__attribute__((noinline)) long aliasing(Node* n, Node** alias, bool c)
{
	Node* first = n->next;
	if (c)
		*alias = nullptr;
	Node* again = n->next;
	return (first ? first->value : -1) * 10 + (again ? again->value : -1);
}

__attribute__((noinline)) long called(Node* n, bool c)
{
	Node* first = n->next;
	if (c)
		relink(&n->next, n);
	Node* again = n->next;
	return first->value * 10 + again->value;
}

__attribute__((noinline)) long bytes(Node* n, unsigned char* raw, bool c)
{
	Node* first = n->next;
	if (c)
		raw[0] = static_cast<unsigned char>(raw[0] ^ 8);
	Node* again = n->next;
	return first == again ? 1 : 2;
}

long g_value = 5;

__attribute__((noinline)) long global_arm(bool c, long x)
{
	const long a = g_value;
	if (c)
		g_value = x;
	return a * 3 + g_value;
}

struct Pair
{
	long a, b;
	long c[4];
};

__attribute__((noinline)) long copy_arm(Pair* p, const Pair* q, bool c)
{
	const long before = p->b + p->c[1];
	if (c)
		*p = *q;
	else
		p->c[1] = 7;
	return before * 100 + p->b + p->c[1];
}

int main()
{
	Body body;
	body.first = { 0, 3, 6 };
	for (uint32_t i = 0; i < 9; ++i)
		body.insts.push_back(Inst{ static_cast<uint8_t>(i % 3), i % 3 == 2 ? ~0u : i + 1, i % 4, (i + 1) % 4 });
	Counter counter(body);
	counter.users_.assign(4, 0);
	counter.run();
	long h = 0;
	for (uint32_t u : counter.users_)
		h = h * 10 + u;
	Node tail = { nullptr, 3 };
	Node mid = { &tail, 2 };
	Node head = { &mid, 1 };
	for (int k = 0; k < 4; ++k)
	{
		Node copy = head;
		h = h * 7 + aliasing(&copy, k & 1 ? &copy.next : &mid.next, k & 2);
		Node again = head;
		h = h * 7 + called(&again, k & 1);
		Node raw = head;
		h = h * 7 + bytes(&raw, reinterpret_cast<unsigned char*>(&raw.next), k & 1);
		h = h * 7 + global_arm(k & 1, k * 11);
		Pair p = { 1, 2, { 3, 4, 5, 6 } };
		const Pair q = { 10, 20, { 30, 40, 50, 60 } };
		h = h * 7 + copy_arm(&p, &q, k & 1);
		h %= 1000000007;
		mid.next = &tail;
	}
	std::printf("%ld\n", h);
	return 0;
}
