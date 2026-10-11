// Reduced from Boost.Container deque_options_t<reservable<true> >.  A nested
// member class template must keep the instantiated enclosing non-type template
// argument visible after a reference-only forward selection is refreshed to the
// class definition.  A dependent base with the same source parameter name must
// not replace the enclosing value.

template<bool Reservable>
struct base {
  static const bool reservable = Reservable;
};

template<bool Reservable>
struct option {
  template<class Base>
  struct pack : Base {
    static const bool reservable = Reservable;
  };
};

typedef option<true>::pack<base<false> > packed;

static_assert(packed::reservable == true, "nested owner value");

int main()
{
  return packed::reservable ? 0 : 1;
}
