// N3485 focus: 3.4.2 [basic.lookup.argdep] associated classes include bases.
// VALIDATION: compile-pass
// Template deduction must see the same ADL set before and after a returned
// class-template specialization has had its reference shape collected.

namespace library {

namespace detail {

struct base
{
};

struct selected
{
  char bytes[2];
};

template<class T>
selected choose(T&)
{
  return selected();
}

}  // namespace detail

template<class T>
struct argument : private detail::base
{
};

template<class T>
argument<T>& make_argument()
{
  return *new argument<T>;
}

template<class T>
int choose(const T&)
{
  return 0;
}

template<class F>
int classify(const F&)
{
  return sizeof(F) == sizeof(detail::selected) ? 0 : 1;
}

int run()
{
  return classify(choose(make_argument<int>()));
}

}  // namespace library

int main()
{
  return 0;
}
