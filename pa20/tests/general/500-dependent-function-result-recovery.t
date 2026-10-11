namespace lib {

// Function-template result recovery must use concrete template arguments even
// when the instantiated function scope still contains local placeholders.

template<class T> struct remove_reference { typedef T type; };
template<class T> struct remove_reference<T&> { typedef T type; };
template<class T> struct remove_reference<T&&> { typedef T type; };
template<class T> using remove_reference_t = typename remove_reference<T>::type;

template<class T>
T&& declval();

template<class T>
remove_reference_t<T>&& move(T&& t) {
  return static_cast<remove_reference_t<T>&&>(t);
}

template<class Iter>
struct unwrap_iter_impl {
  static Iter rewrap(Iter, Iter iter) { return iter; }
  static Iter unwrap(Iter i) { return i; }
};

template<class Iter, bool = false>
struct selected_unwrap_iter_impl;

template<class Iter>
struct selected_unwrap_iter_impl<Iter, false> {
  typedef Iter result_type;
  static result_type unwrap(Iter i) { return i; }
};

template<class Iter, class Impl = selected_unwrap_iter_impl<Iter> >
decltype(Impl::unwrap(declval<Iter>())) selected_unwrap_iter(Iter i) {
  return Impl::unwrap(i);
}

template<class Iter,
         class Unwrapped = decltype(selected_unwrap_iter(declval<Iter>()))>
Unwrapped selected_unwrap_range(Iter first, Iter) {
  return selected_unwrap_iter(first);
}

template<class T>
struct wrapped_iter {
  T value;
};

template<class T>
T to_address(wrapped_iter<T> value) {
  return value.value;
}

template<class Iter, bool = true>
struct address_unwrap_impl;

template<class Iter>
struct address_unwrap_impl<Iter, true> {
  typedef decltype(to_address(declval<Iter>())) result_type;
  static result_type unwrap(Iter value) { return to_address(value); }
};

template<class Iter, class Impl = address_unwrap_impl<Iter> >
decltype(Impl::unwrap(declval<Iter>())) address_unwrap_iter(Iter value) {
  return Impl::unwrap(value);
}

template<class Iter, class Impl = unwrap_iter_impl<Iter>, int = 0>
decltype(Impl::unwrap(declval<Iter>())) unwrap_iter(Iter i) {
  return Impl::unwrap(i);
}

template<class OrigIter, class Iter, class Impl = unwrap_iter_impl<OrigIter> >
OrigIter rewrap_iter(OrigIter orig, Iter iter) {
  return Impl::rewrap(move(orig), move(iter));
}

template<class A, class B>
struct pair {
  A first;
  B second;
  pair(A a, B b) : first(a), second(b) {}
};

template<class A, class B>
pair<A, B> make_pair(A a, B b) {
  return pair<A, B>(a, b);
}

template<class Iter, class Unwrapped = decltype(unwrap_iter(declval<Iter>()))>
pair<Unwrapped, Unwrapped> unwrap_range(Iter first, Iter last) {
  return make_pair(unwrap_iter(move(first)), unwrap_iter(move(last)));
}

template<class Iter, class Unwrapped>
Iter rewrap_range(Iter orig, Unwrapped iter) {
  return rewrap_iter(move(orig), move(iter));
}

struct copy_impl {
  template<class InIter, class Sent, class OutIter>
  pair<InIter, OutIter> operator()(InIter first, Sent last, OutIter result) const {
    while (first != last) {
      *result = *first;
      ++first;
      ++result;
    }
    return make_pair(move(first), move(result));
  }
};

template<class Algorithm, class InIter, class Sent, class OutIter, int = 0>
pair<InIter, OutIter> copy_unwrap_iters(InIter first, Sent last, OutIter out_first) {
  auto range = unwrap_range(first, move(last));
  auto result = Algorithm()(move(range.first), move(range.second), unwrap_iter(out_first));
  return make_pair(rewrap_range<Sent>(move(first), move(result.first)),
                   rewrap_iter(move(out_first), move(result.second)));
}

} // namespace lib

int src[2] = {1, 2};
int dst[2];

int main() {
  lib::copy_unwrap_iters<lib::copy_impl>(src, src + 2, dst);
  lib::selected_unwrap_range(src, src + 2);
  lib::wrapped_iter<int *> wrapped = {src};
  lib::address_unwrap_iter(wrapped);
  return dst[1] - 2;
}
