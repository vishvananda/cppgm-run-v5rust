template <class T>
struct AtomicBase
{
  typedef T int_type;

  operator int_type() const
  {
    return 7;
  }
};

template <class T>
struct Atomic;

template <>
struct Atomic<char> : AtomicBase<char>
{
  typedef char integral_type;
  typedef AtomicBase<char> base_type;

  using base_type::operator integral_type;
};

int use(const Atomic<char> & value)
{
  return value;
}
