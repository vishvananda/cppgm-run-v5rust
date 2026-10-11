// VALIDATION: compile-pass
// A concrete member function-template specialization whose defaulted SFINAE
// parameter still has no concrete type must be dropped even if a stale
// declaration binding is already present in the direct function set.

template<bool B, class T = void>
struct enable_if {
};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

template<bool B, class T = void>
struct disable_if {
  typedef T type;
};

template<class T>
struct disable_if<true, T> {
};

struct input_iterator {
};

template<class T>
struct is_input_iterator {
  static const bool value = false;
};

template<>
struct is_input_iterator<input_iterator> {
  static const bool value = true;
};

template<class C>
struct string_like {
  typedef const C * const_iterator;

  template<class InputIter>
  C *insert(const_iterator, InputIter, InputIter,
            typename disable_if<is_input_iterator<InputIter>::value>::type * = 0);

  template<class ForwardIter>
  C *insert(const_iterator, ForwardIter, ForwardIter,
            typename enable_if<is_input_iterator<ForwardIter>::value>::type * = 0);
};

template<class C>
template<class InputIter>
C *string_like<C>::insert(const_iterator, InputIter, InputIter,
                          typename disable_if<is_input_iterator<InputIter>::value>::type *)
{
  return (C *)1;
}

template<class C>
template<class ForwardIter>
C *string_like<C>::insert(const_iterator, ForwardIter, ForwardIter,
                          typename enable_if<is_input_iterator<ForwardIter>::value>::type *)
{
  return (C *)0;
}

template<class String>
int run_insert(String& s)
{
  input_iterator first;
  input_iterator last;
  typename String::const_iterator p = 0;
  return s.insert(p, first, last) == 0 ? 0 : 1;
}

int main()
{
  string_like<char> s;
  return run_insert(s);
}
