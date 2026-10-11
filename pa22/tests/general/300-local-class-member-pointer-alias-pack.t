// A class alias that expands a parameter pack must preserve the semantic
// identity of a substituted function-local member-pointer owner.

template<class Owner, class Return, class... Args>
struct make_member_pointer {
  using type = Return (Owner::*)(Args...);
};

template<class Owner, class... Args>
using member_pointer =
    typename make_member_pointer<Owner, int, Args...>::type;

template<class T>
int check() {
  member_pointer<T, int> value = nullptr;
  return value == nullptr ? 0 : 1;
}

int main() {
  struct local {
    int call(int);
  };

  return check<local>();
}
