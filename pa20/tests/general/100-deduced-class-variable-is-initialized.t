// `auto p = E();` and `E p = E();` are the same initialization and have to
// produce the same object.  The placeholder initializer is analyzed once to
// deduce the type, at which point there is nothing to initialize yet, so a
// class prvalue stays a temporary; once the type is known the variable still
// has to be initialized from it.
//
// libc++ writes this in min_element, lower_bound and find_end as
// `auto __proj = __identity();`, which is where it was found -- but it is a
// plain C++11 declaration and has nothing to do with the library.

int constructions = 0;
int calls = 0;

struct Counted
{
	int v;
	Counted() : v(7) { ++constructions; }
};

struct Plain { int v; };

Plain make()
{
	++calls;
	Plain p;
	p.v = 3;
	return p;
}

struct Empty { };

int main()
{
	// The deduced variable is initialized, and exactly once.
	auto counted = Counted();
	// The initializer's side effects happen once, not once per analysis.
	auto from_call = make();
	// An empty class has no members to check, so this one only has to compile
	// and lower; it is the shape libc++ actually uses.
	auto empty = Empty();
	(void)empty;
	// The spelled-out form is the control: it always worked.
	Counted spelled = Counted();

	return (counted.v == 7 && spelled.v == 7 && constructions == 2 &&
		from_call.v == 3 && calls == 1) ? 0 : 1;
}
