// VALIDATION: compile-pass
// N3485 focus: 14.5.5 [temp.class.spec], 14.8.2.4 [temp.deduct.partial]

template<int I, class T = void>
struct function_action {};

template<int I, class T>
struct action;

template<class T>
struct mapper {
  typedef int type;
};

template<class Act, class Args>
struct lambda_functor_base;

template<class Act, class Args>
struct lambda_functor_base<action<1, Act>, Args> {
  explicit lambda_functor_base(Args) {}
};

template<class T>
struct lambda_functor {
  lambda_functor(T) {}
};

template<class Result>
const lambda_functor<
  lambda_functor_base<
    action<1, function_action<1, Result> >,
    typename mapper<Result(&)()>::type
  >
>
bind(Result(& a1)()) {
  return lambda_functor_base<
    action<1, function_action<1, Result> >,
    typename mapper<Result(&)()>::type
  >(0);
}

int target();

int main()
{
  bind(target);
}
