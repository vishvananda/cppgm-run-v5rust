// VALIDATION: compile-pass
// N3485 focus: 13.3.3.2 [over.ics.rank], 14.8.2 [temp.deduct]
// A forwarding constructor binds a mutable lvalue without adding const and
// therefore beats an ordinary constructor taking const T &.

template<class T>
struct reference
{
  T * pointer;

  reference(T & value)
    : pointer(&value)
  {
  }
};

template<class T>
struct result
{
  reference<T> stored;

  result(const T & value)
    : stored(value)
  {
  }

  static int accepts_lvalue(T &);

  template<class U>
  result(U && value,
         decltype(accepts_lvalue(static_cast<U &&>(value))) * = 0)
    : stored(static_cast<U &&>(value))
  {
  }
};

int main()
{
  int value = 42;
  result<int> wrapped(value);
  return *wrapped.stored.pointer == 42 ? 0 : 1;
}
