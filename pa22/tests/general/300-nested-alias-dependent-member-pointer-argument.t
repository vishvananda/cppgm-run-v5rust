// Substitution of a nested alias's dependent member-pointer template argument
// must retain the structured C::* declarator around the substituted data type.

template<class T>
struct remove_reference {
  using type = T;
};

template<class T>
struct remove_reference<T &> {
  using type = T;
};

template<bool, class True, class False>
struct select {
  using type = True;
};

template<class True, class False>
struct select<false, True, False> {
  using type = False;
};

template<class T>
struct nested_alias_owner {
  template<class C,
           class U = T,
           class K = typename remove_reference<U>::type>
  using apply = typename select<false, void, K C::*>::type;
};

template<class A, class B>
struct same {
  static constexpr bool value = false;
};

template<class A>
struct same<A, A> {
  static constexpr bool value = true;
};

struct owner {};

static_assert(
    same<nested_alias_owner<int &>::apply<owner>, int owner::*>::value,
    "member data pointer");
static_assert(
    same<nested_alias_owner<int (* const &)()>::apply<owner>,
         int (* const owner::*)()>::value,
    "member data pointer containing a function pointer");

int main() {
  return 0;
}
