// Argument-dependent lookup is reentrant: collecting candidates for one call
// can run a whole second lookup, because a candidate's constraint probes an
// expression of its own.  The associated classes are shared scratch, so the
// inner lookup must not leave the outer one holding its leftovers -- the outer
// call would then have lost the very class whose hidden friend it was looking
// for and would fall back to the builtin operator.
//
// libc++ reaches this with `t << std::setfill('0')`: setfill's inserter is a
// hidden friend of __iom_t4, and __is_ostreamable probes `<<` while the
// enclosing `<<` is still collecting.

template <class T> T&& declval() noexcept;
template <bool B, class T = void> struct enable_if_ { };
template <class T> struct enable_if_<true, T> { typedef T type; };
struct false_ { static const bool value = false; };
struct true_  { static const bool value = true; };

struct base_sink { };
struct sink : base_sink { int n; };

struct fill
{
	int width;
	// Reachable only by argument-dependent lookup on `fill`.
	friend sink& operator<<(sink& s, const fill& f) { s.n += f.width; return s; }
};

template <class S, class T, class = void>
struct streamable : false_ { };

// Probing this specialization runs a nested `<<` lookup.
template <class S, class T>
struct streamable<S, T,
	decltype(declval<S&>() << declval<const T&>(), void())> : true_ { };

// The overload whose constraint does the probing, as libc++'s rvalue stream
// inserter does.
template <class S, class T,
	typename enable_if_<streamable<S, T>::value, int>::type = 0>
S&& operator<<(S&& os, const T& x)
{
	os << x;
	return static_cast<S&&>(os);
}

int main()
{
	sink s;
	s.n = 0;
	fill f;
	f.width = 4;
	s << f;
	return s.n == 4 ? 0 : 1;
}
