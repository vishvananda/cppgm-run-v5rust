// VALIDATION: compile-pass
// N3485 focus: 14.5.5 [temp.class.spec], 14.8.2.4 [temp.deduct.partial]

struct null_type {};

template<int N>
struct case_label {};

template<int Arity, class A0, class A1 = null_type>
struct switch_action {};

template<class Action, class Args>
struct lambda_functor_base;

template<int Case0, class Args>
struct lambda_functor_base<switch_action<2, case_label<Case0> >, Args> {
  typedef case_label<Case0> selected_case;
};

template<class T>
struct lambda_functor : T {
  typedef typename T::selected_case selected_case;
};

typedef lambda_functor<
  lambda_functor_base<switch_action<2, case_label<0> >, null_type>
> switch_functor;

switch_functor::selected_case *selected;

int main()
{
  return selected == 0 ? 0 : 1;
}
