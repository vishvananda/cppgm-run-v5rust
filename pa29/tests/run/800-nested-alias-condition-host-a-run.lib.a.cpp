#include "800-nested-alias-condition-host-a-run.helper.h"
template<class A, class B>
struct ge
{
	static const bool value = sizeof(A) >= sizeof(B);
};
template<class T>
struct TraitsA
{
	template<class U, class R = void>
	using compat = typename enable_if<ge<T, U>::value, R>::type;
	template<class U>
	using assign = compat<U, int>;
};
template<class U>
typename TraitsA<int>::template assign<U> f(U)
{
	return 1;
}
int main()
{
	int x = f(1);
	int z = from_b();
	std::printf("%d %d\n", x, z);
	return 0;
}
