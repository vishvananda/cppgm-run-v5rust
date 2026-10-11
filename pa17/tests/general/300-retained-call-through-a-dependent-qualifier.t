// A call whose qualifier is dependent cannot resolve its function-template
// patterns when the enclosing template is first analyzed, so the retained
// lookup for that call records none.  Each instantiation then deduces its own
// specialization and appends it to the retained set, and the next instantiation
// is offered the previous one's argument types with nothing to deduce from.
//
// The patterns have to be resolved from the scope that can see the qualifier.
// libc++'s __move_backward_impl calls _IterOps<_AlgPolicy>::next(__first,
// __last) exactly this way, and every algorithm instantiated after the first
// one saw the first one's iterator type.

struct classic_policy { };

template <class P> struct ops;

template <>
struct ops<classic_policy>
{
	template <class It>
	static It last_of(It, It last) { return last; }
};

template <class P>
struct algorithm
{
	// One call site, replayed by every instantiation of this member template.
	template <class In>
	In operator()(In first, In last) const
	{
		return ops<P>::last_of(first, last);
	}
};

struct tag_a { int v; };
struct tag_b { long v; };

int main()
{
	unsigned long a[2];
	tag_a b[2];
	tag_b c[2];
	const algorithm<classic_policy> run;

	// Three different iterator types through one replayed call site; the
	// second and third must deduce for themselves rather than inherit the
	// first one's specialization.
	unsigned long* ea = run(a, a + 2);
	tag_a* eb = run(b, b + 2);
	tag_b* ec = run(c, c + 2);

	return (ea == a + 2 && eb == b + 2 && ec == c + 2) ? 0 : 1;
}
