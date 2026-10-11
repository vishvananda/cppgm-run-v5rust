template<class T>
struct I {
  typedef int iterator_category;
};

template<class T>
using iter_cat = typename T::iterator_category;

template<class...>
using void_t = void;

struct nat {};

template<class Default, class Void, template<class...> class Op, class... Args>
struct detector {
  typedef Default type;
};

template<class Default, template<class...> class Op, class... Args>
struct detector<Default, void_t<Op<Args...> >, Op, Args...> {
  typedef Op<Args...> type;
};

template<class Default, template<class...> class Op, class... Args>
using detected_or_t = typename detector<Default, void, Op, Args...>::type;

typedef detected_or_t<nat, iter_cat, I<int> > X;
static_assert(sizeof(X) == sizeof(int), "public alias remains available");

class Hidden {
  typedef int iterator_category;
};

template<class T>
int pick(typename T::iterator_category*) { return 1; }
template<class T>
int pick(...) { return 2; }

template<class...> struct pack {};
template<class A, class B, class C = void> struct choice;
template<class A, class... B>
struct choice<A, pack<B...>, void_t<typename A::iterator_category> > {
  static const int value = 1;
};
template<class A, class B>
struct choice<A, B, void> { static const int value = 2; };

static_assert(choice<Hidden, pack<int> >::value == 2,
              "erased alias retains its private access obligation");

int main() {
  return pick<Hidden>(0) != 2;
}
