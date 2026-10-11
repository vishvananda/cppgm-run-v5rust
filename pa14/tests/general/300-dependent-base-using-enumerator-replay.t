template<class T>
struct base
{
  enum { value = 1 };
};

template<class T>
struct derived : base<T>
{
  using base<T>::value;
  enum { width = value };
};

int main()
{
  return derived<int>::width == 1 ? 0 : 1;
}
