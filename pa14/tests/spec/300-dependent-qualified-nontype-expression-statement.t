// VALIDATION: compile-pass
// N3485 focus: 14.6 [temp.res] dependent qualified names without typename

template<class T>
struct Base
{
  static void touch(T &value)
  {
    value = value + 1;
  }
};

template<class T>
struct Derived
{
  typedef Base<T> Inherited;

  void run(T &value)
  {
    Inherited::touch(value);
  }
};

int main()
{
  int value = 4;
  Derived<int> derived;
  derived.run(value);
  return value == 5 ? 0 : 1;
}
