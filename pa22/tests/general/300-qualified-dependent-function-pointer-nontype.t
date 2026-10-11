// VALIDATION: compile-pass
// A qualified function-pointer non-type argument can name a member of a class
// template-id whose own explicit type argument depends on the current
// instantiation.

namespace concepts
{
template<class ModelFn> struct requirement_;

namespace detail
{
template<void (*)()> struct instantiate {};
}

struct failed {};

template<class Model>
struct requirement
{
  static void failed();
};

template<class Model>
struct requirement<failed ************ Model::************>
{
  static void failed();
};

template<class Model>
struct requirement_<void (*)(Model)>
  : requirement<failed ************ Model::************>
{
};

template<class T>
struct AssignableConcept
{
};
}

template<class Type>
struct list_of
{
  typedef Type value_type;

  struct lazy_concept_checked
  {
    typedef concepts::detail::instantiate<
        &concepts::requirement_<
            void (*)(concepts::AssignableConcept<value_type>)>::failed>
        check;
  };

  typedef typename lazy_concept_checked::check type;
};

list_of<int>::type global_check;
