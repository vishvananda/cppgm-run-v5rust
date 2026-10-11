// Reduced from Boost.Proto via Boost.Xpressive.
// `remove_reference<condition>` is an intermediate qualifier in
// `remove_reference<condition>::type::value`; probing it as a boolean trait
// must not prevent evaluating the final qualified type's static value.

template<bool B>
struct bool_ {
  static const bool value = B;
  typedef bool_ type;
};

template<bool B>
bool const bool_<B>::value;

template<class T>
struct remove_reference {
  typedef T type;
};

namespace mpl {
template<bool B, class T, class F>
struct if_c {
  typedef T type;
};

template<class T, class F>
struct if_c<false, T, F> {
  typedef F type;
};
}

struct yes { static const int value = 1; };
struct no { static const int value = 0; };
struct _ {};

template<class A, class B>
struct wrap {
  template<class Expr, class State, class Data>
  struct impl {
    typedef typename B::template impl<Expr, State, Data>::result_type result_type;
  };
};

struct make_bool {
  template<class Expr, class State, class Data>
  struct impl {
    typedef bool_<true> result_type;
  };
};

template<class If, class Then, class Else>
struct if_ {
  template<class Expr, class State, class Data>
  struct impl {
    typedef typename If::template impl<Expr, State, Data>::result_type condition;
    typedef typename mpl::if_c<
        static_cast<bool>(remove_reference<condition>::type::value),
        wrap<_, Then>,
        wrap<_, Else> >::type which;
    typedef typename which::template impl<Expr, State, Data>::result_type result_type;
  };
};

struct make_yes {
  template<class Expr, class State, class Data>
  struct impl { typedef yes result_type; };
};

struct make_no {
  template<class Expr, class State, class Data>
  struct impl { typedef no result_type; };
};

typedef if_<make_bool, make_yes, make_no> choose_transform;
typedef choose_transform::impl<char const &, int, int> selected_impl;
typedef selected_impl::result_type selected_result;

static_assert(selected_result::value == 1,
              "intermediate template-id qualifier should fall through to ::type::value");

int main()
{
  return 0;
}
