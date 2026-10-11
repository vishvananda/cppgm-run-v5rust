// VALIDATION: compile-pass
// N3485 focus: 12.3.2 [class.conv.fct], 14.5.1 [temp.class]

namespace N {
template<class T>
struct Vec {
};
}

template<class T>
struct Lazy {
  N::Vec<T> values;
  operator const N::Vec<T> &() const
  {
    return values;
  }
};

int main()
{
  Lazy<int> lazy;
  (void)sizeof(lazy);
  return 0;
}
