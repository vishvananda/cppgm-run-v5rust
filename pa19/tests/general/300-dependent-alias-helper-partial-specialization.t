// N3485 focus: [temp.class.spec.match] partial specialization matching.
// A partial-specialization pattern that names a dependent alias template-id
// must expand nested alias helpers after substituting the outer alias arguments.

template<class T>
struct hash {
  hash() = delete;
};

template<class T, class D>
struct unique_ptr {
  typedef T *pointer;
};

template<bool B, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

template<bool...>
struct all {
  static const bool value = true;
};

template<class Key, class Hash = hash<Key> >
struct has_enabled_hash {
  static const bool value = true;
};

template<class Type, class>
using enable_hash_helper_imp = Type;

template<class Type, class... Keys>
using enable_hash_helper =
    enable_hash_helper_imp<
        Type,
        typename enable_if<all<has_enabled_hash<Keys>::value...>::value>::type>;

template<class T, class D>
struct hash<enable_hash_helper<unique_ptr<T, D>, typename unique_ptr<T, D>::pointer> > {
  enum { value = 1 };
};

int main()
{
  hash<unique_ptr<int, int> > h;
  (void)h;
  return hash<unique_ptr<int, int> >::value == 1 ? 0 : 1;
}
