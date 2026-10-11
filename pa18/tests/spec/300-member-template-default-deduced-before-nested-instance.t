// N3485 focus: 14.8.2 [temp.deduct] p5, p8; 14.7.1 [temp.inst] p1; 14.5.2 [temp.mem]
// A deduction substitutes its arguments into a member template's default
// template argument for that candidate only. Copying a rev<S*> deduces the
// converting constructor with U = S*, where !is_same<U, It>::value is false
// and the candidate is dropped; rev<rev<S*> >, instantiated afterwards,
// declares the same constructor with a default that still depends on U, so
// naming the class is no error (the shape of libc++ 18's reverse_iterator,
// whose operator-> returns iterator_traits<It>::pointer).

template<bool B, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

template<bool B, class T = void>
using enable_if_t = typename enable_if<B, T>::type;

template<class T, class U>
struct is_same {
  static const bool value = false;
};

template<class T>
struct is_same<T, T> {
  static const bool value = true;
};

template<class It>
struct iterator_traits {
  typedef typename It::pointer pointer;
  typedef typename It::reference reference;
};

template<class T>
struct iterator_traits<T*> {
  typedef T* pointer;
  typedef T& reference;
};

template<class It>
class rev {
  It current;

public:
  typedef typename iterator_traits<It>::pointer pointer;
  typedef typename iterator_traits<It>::reference reference;

  rev() : current() {}
  explicit rev(It x) : current(x) {}
  template<class U, class = enable_if_t<!is_same<U, It>::value> >
  rev(const rev<U>& u) : current(u.base()) {}

  It base() const { return current; }
  reference operator*() const {
    It tmp = current;
    return *--tmp;
  }
  pointer operator->() const { return &operator*(); }
  rev& operator--() {
    ++current;
    return *this;
  }
};

struct S {
  int x;
};

template<class I>
S* arrow(I first) {
  I copy = first;
  return rev<I>(copy).operator->();
}

int main() {
  S d[1] = {{7}};
  S* p = arrow(rev<S*>(d));
  return p == d && p->x == 7 ? 0 : 1;
}
