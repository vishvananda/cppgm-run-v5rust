// VALIDATION: compile-pass
// Constructor and conversion-function templates keep distinct semantic roles
// through their out-of-class definitions.

struct box
{
  template<class T>
  box(T);

  template<class T>
  operator T() const;
};

template<class T>
box::box(T)
{
}

template<class T>
box::operator T() const
{
  return T();
}

int main()
{
  return 0;
}
