// VALIDATION: compile-pass
// A friend function template declaration may be defined at namespace scope
// beside an unrelated hidden non-template friend overload.

struct value
{
  int n;

  friend int read(value item)
  {
    return item.n;
  }

  template<class T>
  friend int read(T *pointer);
};

template<class T>
int read(T *pointer)
{
  return *pointer + 1;
}

int main()
{
  value item = {5};
  int number = 3;
  return read(item) == 5 && read(&number) == 4 ? 0 : 1;
}
