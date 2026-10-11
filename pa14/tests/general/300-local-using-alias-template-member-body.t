template<class T>
struct identity {
  typedef T type;
};

template<class T>
struct box {
  int f()
  {
    using alias = typename identity<T>::type;
    return alias();
  }
};

int main()
{
  box<int> b;
  return b.f();
}
