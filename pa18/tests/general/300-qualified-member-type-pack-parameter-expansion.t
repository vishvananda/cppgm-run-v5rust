// VALIDATION: compile-pass
// A qualified member type that names a class-template parameter pack must be
// substituted once for each concrete pack element in a function parameter.

template<class Value>
struct definition {
  typedef Value pointer;
};

template<class... Definitions>
struct tuple;

template<class Value>
struct tuple<definition<Value>> {
  typedef Value pointer;
  pointer value;
  tuple(pointer input) : value(input) {}
};

template<class Class, class... Definitions>
struct wrapper {
  typedef tuple<Definitions...> tuple_type;
  tuple_type value;
  wrapper(typename Definitions::pointer... inputs) : value(inputs...) {}
};

int main()
{
  typedef definition<int> int_definition;
  wrapper<int, int_definition> outer(9);
  return outer.value.value == 9 ? 0 : 1;
}
