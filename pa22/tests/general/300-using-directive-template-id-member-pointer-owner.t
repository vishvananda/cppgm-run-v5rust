namespace outer {
namespace aux {
template<int N>
struct weighted_tag {
  typedef char (&type)[N];
};
}
}

namespace imported {
struct failed {};

template<bool C>
struct assert_type {
  typedef void *type;
};

template<bool C>
int assertion_failed(typename assert_type<C>::type);

struct assert_ {
  static assert_ const arg;
  enum relations { equal = 1 };
};

outer::aux::weighted_tag<1>::type operator==(assert_, assert_);

template<assert_::relations R, long X, long Y>
struct assert_relation {};
}

namespace outer {
using namespace imported;
}

enum { rel_value = 1 };

enum {
  value = sizeof(outer::assertion_failed<rel_value>(
      (outer::failed ************ (
          outer::assert_relation<
              outer::assert_::relations(sizeof(outer::assert_::arg == outer::assert_::arg)),
              0,
              0>::************)) 0))
};

int main()
{
  return value > 0 ? 0 : 1;
}
