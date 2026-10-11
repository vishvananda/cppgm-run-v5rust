template<class T>
struct outer {
  struct inner;
};

template<class T>
struct outer<T>::inner {
  typedef int reference;
  reference operator*();
};

template<class T>
auto outer<T>::inner::operator*() -> reference
{
  return 7;
}

int main()
{
  outer<int>::inner value;
  return *value == 7 ? 0 : 1;
}
