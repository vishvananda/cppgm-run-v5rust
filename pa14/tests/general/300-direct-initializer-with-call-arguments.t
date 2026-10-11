// N3485 8.2/1: where a statement could be a declaration or an expression, it
// is a declaration only if it can be one.  `holder h(tag(), pass(a), pass(b))`
// looks like a function declaration because the first argument could be a
// parameter of function type -- but `pass` is not a type, so no parameter
// declaration can be made of `pass(a)` and the statement is an initialization.
//
// libc++ writes the same shape in basic_string::__assign_with_sentinel:
// `const basic_string __temp(__init_with_sentinel_tag(), std::move(__first),
// std::move(__last), __alloc_);`.

struct tag { };

struct holder
{
	int v;
	holder(tag, int a, int b) : v(a + b) { }
};

template <class T> T& pass(T& t) { return t; }

int by_value(int x) { return x; }

int main()
{
	int a = 1, b = 2;

	// The call arguments are what make the declaration parse impossible.
	const holder h(tag(), pass(a), pass(b));

	// A non-template call in the same position reads the same way.
	const holder k(tag(), by_value(a), by_value(b));

	// Note the control that is deliberately absent: `holder m(tag(), int(),
	// int());` is the genuine vexing parse -- every argument could be a
	// parameter, so the declaration parse is viable and wins, and both hosts
	// read it as a function declaration.  This change must not disturb that,
	// and does not: it only takes the reading when a declaration is
	// impossible.

	return (h.v == 3 && k.v == 3) ? 0 : 1;
}
