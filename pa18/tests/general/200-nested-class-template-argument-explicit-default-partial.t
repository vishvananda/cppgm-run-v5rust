// VALIDATION: compile-pass
// Resolve a nested class-template argument before choosing a partial specialization.
struct result {};

template<class> class proxy;

template<class> struct traits {
  typedef int scalar_type;
};

template<class V, class = typename traits<V>::scalar_type>
struct deduce {
  typedef V type;
};

template<class V> struct deduce<proxy<V> > {
  typedef result type;
};

result value = deduce<proxy<result>, int>::type();
