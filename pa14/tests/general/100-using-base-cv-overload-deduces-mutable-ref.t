template<class T>
int same_ref(T&, T&)
{
  return 0;
}

struct Base
{
  int value;

  int& get()
  {
    return value;
  }

  const int& get() const
  {
    return value;
  }
};

struct Derived : Base
{
  using Base::get;

  int run(Derived& other)
  {
    return same_ref(get(), other.get());
  }
};

int main()
{
  Derived a;
  Derived b;
  a.value = 1;
  b.value = 2;
  return a.run(b);
}
