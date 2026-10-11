namespace ns {
typedef unsigned long size_t;
}

typedef unsigned char yes_type;
typedef unsigned short no_type;

template <class T>
class has_member_size {
  template <class U, U>
  class check {};

  template <class C>
  static yes_type f(check<ns::size_t (C::*)(void) const, &C::size> *);

  template <class C>
  static no_type f(...);

 public:
  static const bool value = (sizeof(f<T>(0)) == sizeof(yes_type));
};

struct string_like {
  ns::size_t size() const { return 7; }
};

int main() {
  int check[has_member_size<const string_like>::value ? 1 : -1];
  return sizeof(check) == sizeof(int) ? 0 : 1;
}
