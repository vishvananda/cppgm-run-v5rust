struct visitor
{
};

template<class T>
struct const_default
{
  static int value() { return 1; }
};

template<class T>
struct nonconst_default
{
  static int value() { return 2; }
};

template<class T>
struct forwarding_default
{
  static int value() { return 3; }
};

struct keyword
{
  template<class T>
  const_default<T const> operator|(T const &) const
  {
    return const_default<T const>();
  }

  template<class T>
  nonconst_default<T> operator|(T &) const
  {
    return nonconst_default<T>();
  }

  template<class T>
  forwarding_default<T> operator|(T &&) const
  {
    return forwarding_default<T>();
  }
};

int main()
{
  keyword key;
  visitor default_visitor;
  return (key | default_visitor).value() == 2 ? 0 : 1;
}
