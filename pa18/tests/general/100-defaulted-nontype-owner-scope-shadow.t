struct input_tag {};

template<class Container>
struct input_iterator {
  typedef input_tag iterator_category;
};

template<class T>
struct has_iterator_category {
  struct two { char value[2]; };

  template<class X>
  static char test(int, typename X::iterator_category *);

  template<class X>
  static two test(int, ...);

  static const bool value = sizeof(test<T>(0, 0)) == 1;
};

template<class T, bool = has_iterator_category<T>::value>
struct is_input_iterator {
  static const bool value = true;
};

template<class T>
struct is_input_iterator<T, false> {
  static const bool value = false;
};

struct false_type {
  static const bool value = false;
};

template<class T>
struct is_not_input_iterator {
  static const bool value = !is_input_iterator<T>::value;
};

template<class A, class B>
struct or_trait {
  static const bool value = A::value || B::value;
};

template<bool B, class R>
struct disable_if_c {};

template<class R>
struct disable_if_c<false, R> {
  typedef R type;
};

template<class R, class A, class B>
struct disable_if_or : disable_if_c<or_trait<A, B>::value, R> {};

template<class A, class B>
struct is_convertible {
  static const bool value = false;
};

template<class T>
struct container {
  template<class InputIterator>
  int assign(InputIterator, InputIterator,
             typename disable_if_or<
                 void,
                 is_convertible<InputIterator, unsigned long>,
                 is_not_input_iterator<InputIterator> >::type * = 0) {
    return 1;
  }

  template<class ForwardIterator>
  int assign(ForwardIterator, ForwardIterator,
             typename disable_if_or<
                 void,
                 is_convertible<ForwardIterator, unsigned long>,
                 is_input_iterator<ForwardIterator> >::type * = 0) {
    return 2;
  }
};

struct first_type {};
struct second_type {};

template<class Value>
int exercise() {
  container<Value> value;
  input_iterator<container<Value> > iterator;
  return value.assign(iterator, iterator);
}

int main() {
  return exercise<first_type>() == 1 &&
         exercise<second_type>() == 1 ? 0 : 1;
}
