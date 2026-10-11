// N3485 focus: 14.3.1 type template arguments and 14.6.2.1
// current-instantiation lookup.
// A using-directive candidate with the same template name must not hide the
// current class template when the injected current-class template-id is used as
// another template's type argument.
namespace imported {
template<class T>
struct alias_arg {};

typedef alias_arg<unsigned long> imported_arg;

template<class A, class B>
struct map {};
}

namespace local {
using namespace imported;

template<class A, class B>
struct sink {
  typedef A first;
};

template<class I, class M>
struct map {
  typedef sink<map<I, M>, int> self_sink;
  typedef typename self_sink::first self_type;
  int value;
};
}

int main()
{
  local::map<int*, imported::imported_arg> m;
  m.value = 7;
  return m.value - 7;
}
