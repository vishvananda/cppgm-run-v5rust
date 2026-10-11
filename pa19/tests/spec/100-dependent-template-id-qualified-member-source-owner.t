// N3485 focus: 14.5.5 [temp.class.spec]
// A partial-specialization source owner may carry a dependent qualified member
// type whose qualifier path includes a template-id component.

namespace ns {
template <class T>
struct alloc {
  typedef T value_type;
  template <class U>
  struct rebind { typedef alloc<U> other; };
};
}

namespace impl {
template <class Alloc, class = typename Alloc::value_type>
struct traits {
  template <class T>
  struct rebind { typedef typename Alloc::template rebind<T>::other other; };
};
}

namespace ns {
inline namespace v {
template <class CharT, class Traits, class Alloc>
struct box {
  typedef typename impl::traits<Alloc>::template
      rebind<CharT>::other char_alloc;
  typedef impl::traits<char_alloc> alloc_traits;
};
}

template <class T> struct tag {};
template <class T> struct is_fast { static const int value = 0; };

template <class CharT, class Traits, class Alloc>
struct is_fast<tag<box<CharT, Traits, Alloc> > > {
  static const int value = 1;
};

template <class Alloc>
struct tag<box<char, int, Alloc> > {};
}

int main()
{
  return ns::is_fast<ns::tag<ns::box<char, int, ns::alloc<char> > > >::value == 1 ? 0 : 1;
}
