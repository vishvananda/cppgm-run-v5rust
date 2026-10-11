#include "800-nested-alias-condition-host-b-run.helper.h"
template<class A, class B>
struct le
{
	static const bool value = sizeof(A) <= sizeof(B);
};
template<class T>
struct TraitsB
{
	template<class U, class R = void>
	using compat = typename enable_if<le<T, U>::value, R>::type;
	template<class U>
	using assign = compat<U, int>;
};
template<class U>
typename TraitsB<int>::template assign<U> f(U)
{
	return 2;
}
int from_b()
{
	return f(1);
}
