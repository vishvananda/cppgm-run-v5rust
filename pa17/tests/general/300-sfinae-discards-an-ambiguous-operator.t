// An ambiguity is a property of one expression, not of the program.  Inside a
// substitution the expression is simply ill-formed, so the candidate that
// needed it is discarded and the primary template stands; only an ambiguity
// the program actually depends on is an error.
//
// libc++ reads exactly this answer: __is_ostreamable is a partial
// specialization keyed on
// decltype(declval<_Stream>() << declval<_Tp>(), void()), and an ambiguous <<
// is how it reports that a type is not streamable.

struct false_type { static const bool value = false; };
struct true_type { static const bool value = true; };

template <class T> T&& declval() noexcept;

struct S { };
struct L { L(S); };
struct R { R(S); };

// Neither is better: each needs one user-defined conversion from S.
void operator<<(L, int);
void operator<<(R, int);

struct Plain { };
void operator<<(Plain, int);

template <class A, class B, class = void>
struct streamable : false_type { };

template <class A, class B>
struct streamable<A, B, decltype(declval<A>() << declval<B>(), void())>
	: true_type { };

int main()
{
	int score = 0;
	// Ambiguous, so the partial specialization is discarded and this stays 0.
	if (streamable<S, int>::value) score += 1;
	// Unambiguous, so it is selected -- the discard above must not be
	// something that happens to every operator.
	if (streamable<Plain, int>::value) score += 2;
	return score == 2 ? 0 : 1;
}
