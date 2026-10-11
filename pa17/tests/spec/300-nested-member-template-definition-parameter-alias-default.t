// VALIDATION: compile-pass
// N3485 focus: 14.5.2 [temp.mem]

template<bool Value>
struct enable;

template<>
struct enable<true>
{
  typedef void type;
};

template<class T>
struct adaptor
{
  template<bool>
  struct range;
};

template<class T>
template<bool Mutable>
struct adaptor<T>::range
{
  range()
    : value(Mutable ? 1 : 0)
  {
  }

  template<bool Other = Mutable,
           typename Guard = typename enable<!Other>::type>
  range(range<true> const & other, Guard * = nullptr)
    : value(Other ? 2 : other.value)
  {
  }

  int value;
};

typedef adaptor<int>::range<true> mutable_range;
typedef adaptor<int>::range<false> const_range;

const_range convert(mutable_range const & source)
{
  return source;
}

int main()
{
  mutable_range source;
  return convert(source).value != 1;
}
