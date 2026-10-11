template<class T, T V>
struct integral_constant {
  static constexpr T value = V;
};

typedef integral_constant<bool, true> true_type;
typedef integral_constant<bool, false> false_type;

template<bool, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

template<bool B, class T = void>
using __enable_if_t = typename enable_if<B, T>::type;

template<class A, class B>
struct is_same : false_type {};

template<class A>
struct is_same<A, A> : true_type {};

template<class T>
struct is_trivially_copyable : true_type {};

template<bool B, class If, class Then>
struct conditional;

template<class If, class Then>
struct conditional<true, If, Then> {
  typedef If type;
};

template<class If, class Then>
struct conditional<false, If, Then> {
  typedef Then type;
};

template<bool B, class If, class Then>
using conditional_t = typename conditional<B, If, Then>::type;

template <class T, class = void>
struct trait : is_trivially_copyable<T> {};

template <class T>
struct trait<T, __enable_if_t<is_same<T, typename T::__trivially_relocatable>::value> > : true_type {};

template<class Self, class T, class Alloc>
struct Layout {
  using pointer = T*;
  using allocator_type = Alloc;
  using size_type = int;
};

template<class T, class Alloc, template<class, class, class> class LayoutT>
struct Split : LayoutT<Split<T, Alloc, LayoutT>, T, Alloc> {
  using Base = LayoutT<Split<T, Alloc, LayoutT>, T, Alloc>;
  using typename Base::pointer;
  using typename Base::allocator_type;
  using typename Base::size_type;
  using reloc = conditional_t<trait<pointer>::value && trait<allocator_type>::value, Split, void>;
  void f(size_type n);
};

template<class T, class Alloc, template<class, class, class> class LayoutT>
void Split<T, Alloc, LayoutT>::f(size_type n) {}

int main() {
  return 0;
}
