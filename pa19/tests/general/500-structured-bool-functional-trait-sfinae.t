namespace cpp_decl {
struct Declarator {};
}

namespace ns {
template<class T>
struct default_delete {};
}

struct true_type {
  static const bool value = true;
};

template<class T, class U>
struct is_assignable : true_type {};

template<bool B, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

template<class T, class U,
         class = typename enable_if<bool(is_assignable<T &, U>::value)>::type>
struct gate {
  static const bool value = true;
};

static_assert(gate<
    ns::default_delete<cpp_decl::Declarator>,
    const ns::default_delete<cpp_decl::Declarator> &>::value, "");

int main()
{
  return 0;
}
