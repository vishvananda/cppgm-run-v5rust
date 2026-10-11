// A partial specialization's own pack is bound to a generated shape
// placeholder while its canonical arguments are materialized, so at that point
// the pack has no elements to count.  sizeof... over it is value-dependent
// there rather than a name that is not a pack.
//
// The alias template matters: naming a class template's ::type builds the
// argument through the class, while an alias template substitutes into the
// argument list directly, which is what puts the sizeof... on the shape path.
// libc++'s tuple constrains one pack against another with
// __enable_if_t<sizeof...(_Up) == sizeof...(_Tp)> in exactly this position.

template <bool B, class T = void> struct enable_if_ { };
template <class T> struct enable_if_<true, T> { typedef T type; };

template <bool B, class T = void>
using enable_if_t_ = typename enable_if_<B, T>::type;

template <class... U> struct pack { };

struct no  { static const bool value = false; };
struct yes { static const bool value = true; };

template <class... T>
struct outer
{
	template <class Other, class = void>
	struct inner : no { };

	template <class... U>
	struct inner<pack<U...>, enable_if_t_<sizeof...(U) == sizeof...(T)> >
		: yes { };
};

int main()
{
	// Equal lengths select the partial specialization; unequal lengths leave
	// the primary standing, so the count still has to be right once the shape
	// is instantiated for real.
	int score = 0;
	if (outer<int, char>::inner<pack<long, short> >::value) score += 1;
	if (!outer<int, char>::inner<pack<long> >::value) score += 2;
	if (!outer<int, char>::inner<pack<long, short, int> >::value) score += 4;
	if (outer<int>::inner<pack<long> >::value) score += 8;
	return score == 15 ? 0 : 1;
}
