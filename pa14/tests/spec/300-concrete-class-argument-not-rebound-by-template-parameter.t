// VALIDATION: compile-pass
// N3485 focus: 14.6 [temp.res], 14.7.1 [temp.inst]

template<class R, class F>
struct result_traits
{
  typedef R type;
};

struct F
{
  int value;
};

F make_f()
{
  F value = {7};
  return value;
}

int main()
{
  result_traits<F, F (*)()>::type value = make_f();
  return value.value == 7 ? 0 : 1;
}
