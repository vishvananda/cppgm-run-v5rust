// VALIDATION: compile-pass
// N3485 focus: 11.4 [class.protected], 14.5.4 [temp.friend]

template<class T> struct base {
protected:
  typedef int type;
public:
  template<class U>
  static typename U::type get(U&) { return 0; }
};

template<class T> struct middle : base<T> {
  template<class> friend struct base;
};

template<class T> struct derived : middle<T> {};

int main() {
  derived<int> value;
  return base<char>::get(value);
}
