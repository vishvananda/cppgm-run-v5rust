template<class T>
struct abstract_array_probe
{
  template<class U>
  static double check(U (*)[1]);

  template<class U>
  static char check(...);

  enum
  {
    value = sizeof(abstract_array_probe<T>::template check<T>(0)) == sizeof(char)
  };
};

struct abstract_array_concrete
{
  virtual void f()
  {
  }
};

struct abstract_array_abstract
{
  virtual void f() = 0;
};

int main()
{
  if(abstract_array_probe<abstract_array_concrete>::value)
  {
    return 1;
  }
  return abstract_array_probe<abstract_array_abstract>::value ? 0 : 2;
}
