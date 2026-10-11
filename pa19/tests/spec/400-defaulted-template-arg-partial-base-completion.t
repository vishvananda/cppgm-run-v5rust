// VALIDATION: compile-pass
// N3485 focus: 14.5.5 [temp.class.spec], 14.7.1 [temp.inst]

struct null_type {};

template<class T0 = null_type, class T1 = null_type, class T2 = null_type,
         class T3 = null_type, class T4 = null_type, class T5 = null_type,
         class T6 = null_type, class T7 = null_type, class T8 = null_type,
         class T9 = null_type>
struct tuple {
  typedef T0 head_type;
  explicit tuple(T0) {}
};

struct protect_action {};

template<class T>
struct identity {
  explicit identity(T) {}
  template<class SigArgs> struct sig { typedef T type; };
};

template<class Action, class Args>
struct lambda_functor_base;

template<class Args>
struct lambda_functor_base<protect_action, Args> {
  explicit lambda_functor_base(Args) {}
  template<class SigArgs> struct sig { typedef typename Args::head_type type; };
};

template<class T>
struct lambda_functor : T {
  typedef T inherited;
  lambda_functor(T t) : inherited(t) {}
  typedef typename inherited::template sig<null_type>::type nullary_return_type;
};

template<class T>
lambda_functor<identity<const T> > constant(const T& t)
{
  return identity<const T>(t);
}

template<class Arg>
const lambda_functor<
  lambda_functor_base<
    protect_action,
    tuple<lambda_functor<Arg> >
  >
>
protect(const lambda_functor<Arg>& a1)
{
  return lambda_functor_base<
    protect_action,
    tuple<lambda_functor<Arg> >
  >(tuple<lambda_functor<Arg> >(a1));
}

int main()
{
  protect(constant(2));
}
