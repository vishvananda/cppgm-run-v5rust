template<class T>
struct holder
{
  static bool key(T);

  template<class F> bool visit(F) const { return true; }

  friend bool equal(holder const& value)
  {
    return value.visit([&](T* pointer) { return key(*pointer); });
  }
};

bool use(holder<int> const& value) { return equal(value); }
