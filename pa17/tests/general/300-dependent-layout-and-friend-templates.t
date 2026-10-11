template<class T>
class basic_string;

template<class T>
struct Holder {
  template<class U>
  friend class basic_string;

  typedef T value_type;
  value_type* ptr;
  char pad[sizeof(value_type*)];
  int d;

  enum {
    k = sizeof(value_type) > 1 ? sizeof(value_type) : 1
  };
};

template<class C, class Traits>
struct S {
  class Proxy {
    friend class S;
  };

  bool equal(const S&) const {
    return true;
  }
};

template<class C, class Traits>
inline bool operator==(const S<C, Traits>& a, const S<C, Traits>& b) {
  return a.equal(b);
}

typedef Holder<char> holder_char;
typedef S<int, int> string_like;

int main() {
  return 0;
}
