// Candidate collection is reentrant: a candidate's constraint can probe an
// expression that collects candidates of its own.  The marks that keep a
// function from being collected twice are stamped with a shared generation,
// so a nested collection leaves the outer one's marks looking unset -- and the
// same function is then added a second time and ties with itself, which is
// reported as an ambiguity between two identical signatures.
//
// libc++ reaches this whenever a type has both a conversion to its own enum
// and its own stream inserter, with __is_ostreamable probing `<<` in between.

template <class T> T&& declval() noexcept;
template <bool B, class T = void> struct enable_if_ { };
template <class T> struct enable_if_<true, T> { typedef T type; };
struct false_ { static const bool value = false; };
struct true_  { static const bool value = true; };

struct sink { int n; };

// Both a conversion to its enum and its own inserter, so ordinary lookup and
// argument-dependent lookup can each reach the same function.
struct op_t
{
	enum Kind { none = 0, add = 1 };
	Kind kind;
	op_t() : kind(none) { }
	op_t(Kind value) : kind(value) { }
	operator Kind() const { return kind; }
};

sink& operator<<(sink& s, op_t operation)
{
	s.n += static_cast<int>(operation.kind);
	return s;
}

template <class S, class T, class = void>
struct streamable : false_ { };

// Probing this runs a nested collection while the outer one is in progress.
template <class S, class T>
struct streamable<S, T,
	decltype(declval<S&>() << declval<const T&>(), void())> : true_ { };

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
	op_t o(op_t::add);
	s << o;
	return s.n == 1 ? 0 : 1;
}
