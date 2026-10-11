// VALIDATION: compile-pass
// N3485 focus: 14.5.5 [temp.class.spec], 14.7.1 [temp.inst]

struct null_type {};

template<class T0 = null_type, class T1 = null_type>
struct tuple {
  explicit tuple(T0, T1) {}
};

struct default_label {};
template<int N> struct case_label {};

template<int Arity,
         class Switch1 = null_type,
         class Switch2 = null_type,
         class Switch3 = null_type,
         class Switch4 = null_type,
         class Switch5 = null_type,
         class Switch6 = null_type,
         class Switch7 = null_type,
         class Switch8 = null_type,
         class Switch9 = null_type>
struct switch_action {};

template<class Action, class Args>
struct lambda_functor_base;

template<class Args>
struct lambda_functor_base<switch_action<2, default_label>, Args> {
  explicit lambda_functor_base(Args) {}
  typedef default_label selected_case;
};

template<class T>
struct lambda_functor : T {
  typedef T inherited;
  lambda_functor(T t) : inherited(t) {}
  typedef typename inherited::selected_case selected_case;
};

template<class Tag, class Lambda>
struct tagged_lambda_functor {
  Lambda lambda;
  explicit tagged_lambda_functor(const Lambda& l) : lambda(l) {}
};

template<class Type>
struct switch_case_tag {};

struct placeholder {
  typedef null_type selected_case;
};
struct increment_action {};
template<class Act> struct pre_increment_decrement_action {};
template<class T> struct identity {};

template<class Args>
struct lambda_functor_base<pre_increment_decrement_action<increment_action>, Args> {
  explicit lambda_functor_base(Args) {}
  typedef null_type selected_case;
};

template<class Arg>
tagged_lambda_functor<switch_case_tag<default_label>, lambda_functor<Arg> >
default_statement(const lambda_functor<Arg>& a)
{
  return tagged_lambda_functor<
    switch_case_tag<default_label>,
    lambda_functor<Arg>
  >(a);
}

template<class TestArg, class TagData0, class Arg0>
lambda_functor<
  lambda_functor_base<
    switch_action<2, TagData0>,
    tuple<lambda_functor<TestArg>, Arg0>
  >
>
switch_statement(const lambda_functor<TestArg>& a0,
                 const tagged_lambda_functor<switch_case_tag<TagData0>, Arg0>& a1)
{
  return lambda_functor_base<
    switch_action<2, TagData0>,
    tuple<lambda_functor<TestArg>, Arg0>
  >(tuple<lambda_functor<TestArg>, Arg0>(a0, a1.lambda));
}

typedef lambda_functor<
  lambda_functor_base<
    pre_increment_decrement_action<increment_action>,
    tuple<identity<int>, null_type>
  >
> increment_functor;

template<class T>
typename lambda_functor<T>::selected_case *selected_case_of(lambda_functor<T>)
{
  return 0;
}

typedef lambda_functor<placeholder> placeholder_functor;
placeholder_functor placeholder_value = placeholder_functor(placeholder());
increment_functor increment_value = increment_functor(
  lambda_functor_base<
    pre_increment_decrement_action<increment_action>,
    tuple<identity<int>, null_type>
  >(tuple<identity<int>, null_type>(identity<int>(), null_type())));

int main()
{
  return selected_case_of(
      switch_statement(placeholder_value, default_statement(increment_value))) ==
      0 ? 0 : 1;
}
