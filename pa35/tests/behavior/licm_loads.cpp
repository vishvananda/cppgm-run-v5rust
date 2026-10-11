// Loop load promotion (8.7): loads of an invariant address inside a loop
// read a slot the loop keeps current. Cases the pass must get right:
// a store to exactly the address refills it; a store that may alias it
// invalidates it (and reloads only where the loop would load next); a
// call on a cold path invalidates it; a pointer that is not known
// readable is never loaded where the loop would not have (a zero-trip
// loop, an exit before the first read, a call freeing it before a return);
// a value that becomes an inner loop's phi flows out to an outer header's
// phi through the outer latch; unions and byte stores through other types
// alias everything.
#include <cstdio>
#include <cstdint>
#include <cstdlib>
#include <cstring>

struct Vec
{
	int* begin;
	int* end;
	int* cap;
};

// `v.begin` is invariant but the store through `out` may alias it: the
// reload after the store keeps the value right when they do alias.
__attribute__((noinline)) long sum_into(const Vec& v, int* out, int n)
{
	long total = 0;
	for (int i = 0; i < n; ++i)
	{
		total += v.begin[i & 3];
		*out = static_cast<int>(total);
	}
	return total;
}

struct Counter
{
	unsigned count;
	unsigned* table;
};

// `c.table` is reloaded after `bump` (a call may change it) only where the
// loop reads it next; `bump` really does change it on the third call.
unsigned storage[2][8];
__attribute__((noinline)) void bump(Counter& c, unsigned k)
{
	if (k == 2)
		c.table = storage[1];
	++c.count;
}

__attribute__((noinline)) unsigned walk(Counter& c, const unsigned* keys, unsigned n)
{
	unsigned acc = 0;
	for (unsigned i = 0; i < n; ++i)
	{
		acc += c.table[keys[i] & 7];
		if (keys[i] & 8)
			bump(c, i);
	}
	return acc + c.count;
}

// A raw pointer read only inside the loop: a zero-trip loop must not read
// it (it is null then).
__attribute__((noinline)) int first_or(const int* p, int n, int fallback)
{
	int r = fallback;
	for (int i = 0; i < n; ++i)
		r += *p + i;
	return r;
}

// A pointer freed by a call before the loop returns: no load may follow
// the call on that path.
struct Node
{
	int value;
	Node* next;
};

__attribute__((noinline)) int consume(Node* head)
{
	int total = 0;
	while (head != nullptr)
	{
		total += head->value;
		Node* next = head->next;
		free(head);
		head = next;
	}
	return total;
}

// A store to exactly the promoted address: the slot takes the stored
// value, and the loop sees it on the next pass.
struct State
{
	int limit;
	int steps;
};

__attribute__((noinline)) int tighten(State& s, const int* data, int n)
{
	int hits = 0;
	for (int i = 0; i < n; ++i)
	{
		if (data[i] > s.limit)
		{
			s.limit = data[i];
			++hits;
		}
		++s.steps;
	}
	return hits * 100 + s.limit + s.steps;
}

// Nested loops: the inner loop's promoted value (a phi after the inner
// loop) reaches the outer header's phi through the outer latch, and the
// inner loop (3 trips, branching) is fully unrolled at -O3.
struct Row
{
	unsigned cells[3];
	unsigned flag;
};

__attribute__((noinline)) unsigned fold_rows(const Row* rows, unsigned n, unsigned* sink)
{
	unsigned acc = 0;
	for (unsigned r = 0; r < n; ++r)
	{
		for (unsigned k = 0; k < 3; ++k)
		{
			if (rows[r].cells[k] & 1u)
				sink[k] += rows[r].cells[k];   // may alias `rows` (no, but the loop cannot tell)
			acc += sink[k] ^ rows[r].flag;
		}
	}
	return acc;
}

// A union: the float write must be seen by the integer read.
union Bits
{
	float f;
	uint32_t u;
};

__attribute__((noinline)) uint32_t pun(Bits& b, int n)
{
	uint32_t acc = 0;
	for (int i = 0; i < n; ++i)
	{
		b.f = static_cast<float>(i) * 0.5f;
		acc ^= b.u;
	}
	return acc;
}

// A byte store into the middle of a wider field the loop reads.
struct Packed
{
	uint32_t word;
	uint8_t tail[4];
};

__attribute__((noinline)) uint32_t poke(Packed& p, int n)
{
	uint32_t acc = 0;
	for (int i = 0; i < n; ++i)
	{
		reinterpret_cast<uint8_t*>(&p.word)[i & 3] = static_cast<uint8_t>(i);
		acc += p.word;
	}
	return acc;
}

// A global this unit defines: readable, so the preheader may fill the
// slot even though the loop reads it after an early exit test.
int global_scale = 3;
int global_sum = 0;
__attribute__((noinline)) int scale_all(const int* a, int n)
{
	int acc = 0;
	for (int i = 0; i < n; ++i)
	{
		if (a[i] < 0)
			break;
		acc += a[i] * global_scale;
		global_sum += acc;   // a store elsewhere: not to global_scale
	}
	return acc;
}

int main()
{
	int data[8] = { 1, 2, 3, 4, 5, 6, 7, 8 };
	Vec v = { data, data + 8, data + 8 };
	int out = 0;
	const long plain = sum_into(v, &out, 6);
	printf("sum_into %ld %d\n", plain, out);
	// The output aliases the vector's first element: the reload matters.
	Vec w = { data, data + 8, data + 8 };
	const long aliased = sum_into(w, data, 5);
	printf("sum_into aliased %ld %d\n", aliased, data[0]);

	for (unsigned k = 0; k < 8; ++k)
	{
		storage[0][k] = k * 10;
		storage[1][k] = k * 1000;
	}
	Counter c = { 0, storage[0] };
	unsigned keys[6] = { 1, 9, 2, 10, 3, 4 };
	printf("walk %u\n", walk(c, keys, 6));

	printf("first_or %d %d\n", first_or(nullptr, 0, 7), first_or(data, 3, 1));

	Node* head = nullptr;
	for (int i = 0; i < 5; ++i)
	{
		Node* n = static_cast<Node*>(malloc(sizeof(Node)));
		n->value = i * i;
		n->next = head;
		head = n;
	}
	printf("consume %d\n", consume(head));

	State s = { 3, 0 };
	int rising[6] = { 1, 5, 2, 9, 9, 10 };
	printf("tighten %d\n", tighten(s, rising, 6));

	Row rows[3] = { { { 1, 2, 3 }, 7 }, { { 4, 5, 6 }, 1 }, { { 7, 8, 9 }, 2 } };
	unsigned sink[3] = { 0, 0, 0 };
	const unsigned folded = fold_rows(rows, 3, sink);
	printf("fold_rows %u %u %u %u\n", folded, sink[0], sink[1], sink[2]);

	Bits b;
	b.u = 0;
	printf("pun %u\n", pun(b, 5));

	Packed p = { 0x01020304u, { 0, 0, 0, 0 } };
	printf("poke %u\n", poke(p, 6));

	int positives[5] = { 1, 2, -1, 4, 5 };
	const int scaled = scale_all(positives, 5);
	printf("scale_all %d %d\n", scaled, global_sum);
	return 0;
}
