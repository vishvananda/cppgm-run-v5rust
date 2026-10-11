template<class T>
T make_copy(T source)
{
  return source;
}

struct box
{
  int value;

  box() : value(0)
  {
  }

  template<class U>
  box& operator=(U&& rhs)
  {
    value = rhs.value;
    return *this;
  }
};

template<class T>
void assign_rvalue(T const& source)
{
  T target;
  target = make_copy<T>(source);
}

template<class T>
void assign_const_lvalue(T const& source)
{
  T target;
  target = source;
}

int main()
{
  box source;
  source.value = 7;
  assign_rvalue(source);
  assign_const_lvalue(source);
  return 0;
}
