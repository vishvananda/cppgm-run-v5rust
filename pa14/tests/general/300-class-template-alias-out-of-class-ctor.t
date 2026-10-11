template<class T>
struct Box {
  typedef T value_type;
  value_type value;
  Box(const value_type& x);
};

template<class T>
Box<T>::Box(const value_type& x) : value(x) {}

int f() {
  Box<int> b(7);
  return b.value;
}

int main() { return f(); }
