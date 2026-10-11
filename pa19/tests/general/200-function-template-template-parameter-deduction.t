namespace ttp_deduce {

template<class T>
class box
{
public:
  box() {}
  box(const T &) {}

  const box & self() const
  {
    return *this;
  }

  box & operator=(const box &)
  {
    return *this;
  }
};

template<class T, template<class> class U>
U<T> pass(const U<T> x)
{
  return x.self();
}

int test()
{
  box<double> a;
  a = pass(a);
  return 0;
}

}  // namespace ttp_deduce

int main()
{
  return ttp_deduce::test();
}
