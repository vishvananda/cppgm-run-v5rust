// VALIDATION: compile-pass
// N3485 focus: 14.5.5 [temp.class.spec], 14.5.7 [temp.alias],
// 14.8.2 [temp.deduct]
// A non-deduced alias pattern must be rechecked after its template-template
// and pack parameters have been deduced from the other specialization args.

template<class T>
T&& declval();

template<class... T>
struct make_void
{
  typedef void type;
};

template<class... T>
using void_t = typename make_void<T...>::type;

struct false_type
{
  static const bool value = false;
};

struct true_type
{
  static const bool value = true;
};

struct nonesuch {};

template<class Default,
         class AlwaysVoid,
         template<class...> class Operation,
         class... Arguments>
struct detector
{
  typedef false_type value_t;
};

template<class Default,
         template<class...> class Operation,
         class... Arguments>
struct detector<Default,
                void_t<Operation<Arguments...> >,
                Operation,
                Arguments...>
{
  typedef true_type value_t;
};

template<template<class...> class Operation, class... Arguments>
using is_detected =
    typename detector<nonesuch,
                      void,
                      Operation,
                      Arguments...>::value_t;

struct iterator {};

struct range
{
  iterator begin();
};

iterator begin(range&);

template<class... T>
using result_of_begin = decltype(begin(declval<T>()...));

template<template<class...> class Operation, class... Arguments>
using apply = Operation<Arguments...>;

using direct_apply = apply<result_of_begin, range&>;

static_assert(is_detected<result_of_begin, range&>::value,
              "the valid alias operation is detected");

int main()
{
  return is_detected<result_of_begin, range&>::value ? 0 : 1;
}
