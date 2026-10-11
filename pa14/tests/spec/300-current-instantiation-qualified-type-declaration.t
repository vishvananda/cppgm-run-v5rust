// VALIDATION: compile-pass
// N3485 focus: 14.6.2.1 [temp.dep.type] current instantiation

template<class T>
struct Holder
{
  struct Base
  {
    int value;
  };

  typedef Holder<T> self_type;

  int run()
  {
    self_type::Base *p = 0;
    return p == 0 ? 0 : 1;
  }
};

int main()
{
  Holder<int> h;
  return h.run();
}
