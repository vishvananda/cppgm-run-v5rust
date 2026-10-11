// VALIDATION: compile-pass
// N3485 focus: 14.5.5 [temp.class.spec], 14.8.2.5 [temp.deduct.type]

template<int Arity, class Act>
class action;

template<class ThrowType>
struct throw_action;

struct rethrow_action {};

template<class Action, class Args>
class lambda_functor_base;

template<class T>
class lambda_functor;

struct null_type {};

template<class Arg>
struct throws_for_sure_phase2 {
  static const bool value = false;
};

template<int N, class ThrowType, class Args>
struct throws_for_sure_phase2<
  lambda_functor<
    lambda_functor_base<action<N, throw_action<ThrowType> >, Args>
  >
> {
  static const bool value = true;
};

typedef lambda_functor<
  lambda_functor_base<
    action<0, throw_action<rethrow_action> >,
    null_type
  >
> throw_expr;

static_assert(throws_for_sure_phase2<throw_expr>::value, "nested nttp");

int main()
{
  return 0;
}
