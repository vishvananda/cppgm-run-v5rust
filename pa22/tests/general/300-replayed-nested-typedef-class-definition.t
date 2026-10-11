namespace boost {
namespace mpl {

struct assert_ {
};

struct failed {
};

template<bool C>
failed assertion_failed(...);

}
}

struct yes_tag {
  char c[1];
};

struct no_tag {
  char c[2];
};

struct empty_arg_list {
  static no_tag has_key(...);
};

template<class T>
struct tag {
  typedef T key_type;
};

template<class TaggedArg, class Next = empty_arg_list>
struct arg_list : Next {
  typedef typename TaggedArg::key_type key_type;
  typedef arg_list<TaggedArg, Next> self;

  static yes_tag has_key(key_type *);
  using Next::has_key;

  enum { unique = sizeof(Next::has_key((key_type *)0)) == sizeof(no_tag) };

  struct duplicate_keyword;
  typedef struct duplicate_keyword379 : boost::mpl::assert_ {
    static boost::mpl::failed ************ (
        duplicate_keyword::************ assert_arg()) (key_type)
    {
      return 0;
    }
  } mpl_assert_arg379;

  enum {
    mpl_assertion_in_line_379 =
        sizeof(boost::mpl::assertion_failed<unique>(
            mpl_assert_arg379::assert_arg()))
  };

  typedef key_type exposed;
};

template<class T>
struct wrapper {
  typedef arg_list<tag<T> > first;
  typedef arg_list<tag<int>, first> second;
};

template<class T>
struct use_reference_members {
  typedef typename wrapper<T>::second::exposed type;
};

int main() {
  use_reference_members<int>::type value = 0;
  return value;
}
