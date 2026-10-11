// VALIDATION: compile-pass
// Explicit member-template type arguments retained beneath a pack expansion
// must use the concrete owner type for each expanded pack element.

template<class First, class Second>
struct definition {
  typedef First class_type;
  typedef Second func_type;
};

template<class First, class Second>
struct result {
  typedef First type;
};

template<class... Elements>
struct tuple;

template<class Element>
struct tuple<Element> {
  Element value;
  tuple(Element input) : value(input) {}
};

struct provider {
  template<class First, class Second>
  typename result<First, Second>::type get()
  {
    return First();
  }
};

template<class... Definitions>
struct factory {
  template<class... ArgumentsIn>
  static tuple<typename result<
      typename ArgumentsIn::class_type,
      typename ArgumentsIn::func_type>::type...>
  make(provider &input, tuple<ArgumentsIn...> *)
  {
    typedef tuple<typename result<
        typename ArgumentsIn::class_type,
        typename ArgumentsIn::func_type>::type...> result_type;
    return result_type(input.get<typename ArgumentsIn::class_type,
                                 typename ArgumentsIn::func_type>()...);
  }
};

int main()
{
  provider input;
  typedef definition<int, long> int_definition;
  tuple<int> value = factory<int_definition>::make(
      input, static_cast<tuple<int_definition> *>(0));
  return value.value == 0 ? 0 : 1;
}
