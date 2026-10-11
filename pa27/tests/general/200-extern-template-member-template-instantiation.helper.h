template<class T>
struct Box
{
  T value;
  Box(T input) : value(input) {}
  int read() const;
  template<class F> __attribute__((noinline)) int apply(F function) const;
};

template<class T>
int Box<T>::read() const { return value + 10; }

template<class T>
template<class F>
int Box<T>::apply(F function) const { return function(value); }

extern template struct Box<int>;
