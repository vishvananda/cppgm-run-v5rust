template<class... T>
struct sequence {};

namespace meta { struct type_info {}; }

struct library_base {
  template<class U>
  const meta::type_info & get_type_info() const {
    static meta::type_info value;
    return value;
  }
};

struct library : library_base {};

template<class T>
struct imported_class {
  template<class... Args>
  imported_class(sequence<Args...> *, const library &, Args...);

  const meta::type_info & info;
  const meta::type_info & get_type_info() const { return info; }
};

template<class T>
template<class... Args>
imported_class<T>::imported_class(
    sequence<Args...> *, const library & lib, Args...)
  : info(lib.get_type_info<T>()) {}

int main() {
  return 0;
}
