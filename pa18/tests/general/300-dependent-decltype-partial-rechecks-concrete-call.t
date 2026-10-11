template<class T>
T && declval();

template<class T>
struct pointer_traits {
};

template<class Pointer, class = void>
struct helper {
  typedef int type;
};

template<class Pointer>
struct helper<
    Pointer,
    decltype((void)pointer_traits<Pointer>::to_address(
        declval<const Pointer &>()))> {
  typedef char type;
};

struct iterator {
  int * operator->() const;
};

static_assert(sizeof(typename helper<iterator>::type) == sizeof(int), "");

int main()
{
  return 0;
}
