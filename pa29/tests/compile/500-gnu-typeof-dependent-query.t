// PA29 GNU typeof: substitute retained syntax and traverse packs without
// demanding the body of a function merely named in an unevaluated operand.
template<class A, class B> struct same { static const bool value = false; };
template<class A> struct same<A, A> { static const bool value = true; };
template<class T> T query();
template<class T> struct pick { typedef __typeof__(query<T>()) type; };
template<class T> struct nested { typedef int member; };
template<class T> struct qualified { typedef typename __typeof__(query<T>())::member type; };
template<class... T> struct pack {};
template<class... T> struct list { typedef pack<__typeof__(query<T>())...> type; };
static_assert(same<pick<int&>::type, int>::value, "strip expression reference");
static_assert(same<qualified<nested<int>>::type, int>::value, "qualified typeof");
static_assert(same<list<int, long>::type, pack<int, long>>::value, "operand packs");
int main() { return 0; }
