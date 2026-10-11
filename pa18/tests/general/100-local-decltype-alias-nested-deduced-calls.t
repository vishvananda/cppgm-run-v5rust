#include "100-local-decltype-alias-nested-deduced-calls.h"

namespace traits {
template<class T>
struct remove_reference {
  typedef T type;
};

template<class T>
struct remove_reference<T&> {
  typedef T type;
};

template<class T>
struct remove_cv {
  typedef T type;
};
}

namespace library {
namespace detail {
typedef void (*value_tag)(int);

template<class T>
T& deref(T&, value_tag);

template<class T>
struct wrapper {
  typedef T type;
};

template<class T>
wrapper<T> wrap(T&);
}

namespace type_of {
template<class T>
using remove_cv_ref_t =
    typename traits::remove_cv<
        typename traits::remove_reference<T>::type>::type;
}
}

#include "100-local-decltype-alias-nested-deduced-calls-body.h"
