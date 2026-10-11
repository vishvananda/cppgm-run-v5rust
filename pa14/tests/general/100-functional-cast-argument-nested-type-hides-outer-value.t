// VALIDATION: compile-pass
// Boost.Contract reduction: an inner type must hide an outer value when the
// unqualified name is used as a functional cast in a call argument.

namespace outer
{

struct shadow_maker_tag
{
};

shadow_maker_tag shadow_maker;

namespace inner
{

template<class F>
void invoke(F f)
{
  f();
}

struct shadow_maker
{
  int * out;

  explicit shadow_maker(int * p)
    : out(p)
  {
  }

  void operator()()
  {
    *out = 19;
  }
};

int run()
{
  int value = 0;
  invoke(shadow_maker(&value));
  return value;
}

}
}

int main()
{
  return outer::inner::run() == 19 ? 0 : 1;
}
