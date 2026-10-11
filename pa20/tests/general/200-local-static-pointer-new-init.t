// A function-local static POINTER dynamically initialized from a
// new-expression: the guard's init must call operator new and store
// the heap address, not construct the class over the pointer slot.
struct S
{
	long a;
	long b;
	long c;
};

S* get()
{
	static S* p = new S();
	return p;
}

int main()
{
	S* p = get();
	p->b = 42;
	if (p != get())
		return 2;
	return p->a == 0 && p->b == 42 && p->c == 0 ? 0 : 1;
}
