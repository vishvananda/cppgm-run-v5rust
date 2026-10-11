// Typed accesses (IF_TYPED: a pointer access and a wider non-pointer one
// through different bases name different objects): a vector's base kept
// across stores to its int elements, members of one object reached through
// two pointers, a union written and read through the union object, a
// pointer overwritten byte by byte (a character type aliases anything),
// storage reused by placement new as another type, and an object whose
// pointer and integer members share a base.
#include <cstdio>
#include <cstring>
#include <new>
#include <vector>
struct Body
{
	std::vector<unsigned> uses;
	std::vector<int> next;
};
__attribute__((noinline)) unsigned count(Body& b, const std::vector<int>& ops)
{
	unsigned s = 0;
	for (unsigned long i = 0; i < ops.size(); ++i)
	{
		b.uses[static_cast<unsigned>(ops[i]) % b.uses.size()] += 1;
		s += b.uses[i % b.uses.size()];
	}
	return s;
}
struct Node
{
	Node* link;
	long value;
	int* slot;
};
__attribute__((noinline)) long members(Node* a, long* v, int** s)
{
	Node* l = a->link;
	*v = 41;                 // a->value through another pointer
	int* p = a->slot;
	*s = p + 1;              // a->slot through another pointer
	return (l != nullptr) + a->value + (a->slot - p);
}
union Pun
{
	void* p;
	long n;
};
__attribute__((noinline)) long punned(Pun* u, void* q)
{
	u->p = q;
	const long n = u->n;
	u->n = n + 8;
	return reinterpret_cast<char*>(u->p) - reinterpret_cast<char*>(q);
}
__attribute__((noinline)) long bytewise(void** slot, const void* replacement)
{
	void* before = *slot;
	unsigned char* bytes = reinterpret_cast<unsigned char*>(slot);
	const unsigned char* from = reinterpret_cast<const unsigned char*>(&replacement);
	for (unsigned k = 0; k < sizeof(void*); ++k)
		bytes[k] = from[k];
	return reinterpret_cast<const char*>(*slot) - reinterpret_cast<const char*>(before);
}
__attribute__((noinline)) long reused(void* storage)
{
	long* n = new (storage) long(12);
	const long first = *n;
	void** p = new (storage) void*(storage);
	return first + (*p == storage);
}
struct Mixed
{
	int* begin;
	int count;
	int* end;
};
__attribute__((noinline)) long shared(Mixed* m)
{
	long s = 0;
	for (int i = 0; i < m->count; ++i)
	{
		m->begin[i] = i * 3;   // int stores while begin and end are reread
		s += m->end - m->begin;
	}
	return s;
}
int main()
{
	Body b;
	b.uses.assign(7, 0);
	std::vector<int> ops;
	for (int i = 0; i < 50; ++i)
		ops.push_back(i * 13);
	const unsigned c = count(b, ops);
	int arr[4] = { 1, 2, 3, 4 };
	Node n = { &n, 5, arr };
	const long m = members(&n, &n.value, &n.slot);
	Pun u;
	int target[8];
	const long pn = punned(&u, target);
	int a1[2], a2[2];
	void* slot = a1;
	const long bw = bytewise(&slot, a2);
	alignas(8) unsigned char storage[16];
	const long ru = reused(storage);
	int data[10];
	Mixed mx = { data, 10, data + 10 };
	const long sh = shared(&mx);
	std::printf("%u %ld %d %ld %ld %ld %ld %d\n", c, m, n.slot == arr + 1, pn, bw == static_cast<long>(reinterpret_cast<char*>(a2) - reinterpret_cast<char*>(a1)), ru, sh, data[9]);
	return 0;
}
