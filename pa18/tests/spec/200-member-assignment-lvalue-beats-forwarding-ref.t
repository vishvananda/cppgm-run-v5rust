// VALIDATION: compile-pass
// N3485 focus: 14.8.2.4 [temp.deduct.partial], 14.8.3 [temp.over]
// For an lvalue RHS, a member operator=(T&) is more specialized than the
// forwarding-reference operator=(T&&) after deduction collapses both to int&.

template<class T>
struct lvalue_assignment {
  static const int value = 1;
};

template<class T>
struct forwarding_assignment {
  static const int value = 2;
};

struct keyword {
  template<class T>
  lvalue_assignment<T> operator=(T &) const
  {
    return lvalue_assignment<T>();
  }

  template<class T>
  forwarding_assignment<T> operator=(T &&) const
  {
    return forwarding_assignment<T>();
  }
};

int selected(lvalue_assignment<int>)
{
  return 0;
}

int selected(forwarding_assignment<int &>)
{
  return 1;
}

int main()
{
  int value = 0;
  return selected(keyword() = value);
}
