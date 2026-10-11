// Boost.Flyweight reduction: an instantiated class-template member typedef used
// as a qualified-id base must beat an outer namespace with the same name.

namespace boost {
namespace core {
struct namespace_marker {};
}

namespace flyweights {
namespace detail {
template<class T>
struct flyweight_core {
  typedef int handle_type;

  static handle_type insert(T value)
  {
    return value;
  }
};
}

template<class T>
struct flyweight {
  typedef detail::flyweight_core<T> core;
  typedef typename core::handle_type handle_type;

  handle_type h;

  explicit flyweight(T value)
    : h(core::insert(value))
  {
  }
};
}
}

int main()
{
  boost::flyweights::flyweight<int> value(7);
  return value.h - 7;
}
