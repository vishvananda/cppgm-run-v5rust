// VALIDATION: compile-pass
// Default non-type arguments in a member template must resolve qualified
// suffixes through the substituted owner, not through the enclosing class.

template<int N>
struct size_c {
  static const int value = N;
};

struct nil {
  typedef size_c<0> size;
};

template<class Car, class Cdr>
struct cons {
  typedef size_c<Cdr::size::value + 1> size;
};

template<class Context>
struct iterator {
  typedef Context context_type;

  template<class It1, class It2,
           int Size1 = It1::context_type::size::value,
           int Size2 = It2::context_type::size::value>
  struct defaults {
    static const int first = Size1;
    static const int second = Size2;
  };

  template<class It1, class It2,
           int Size1 = It1::context_type::size::value,
           int Size2 = It2::context_type::size::value>
  struct equal_to {
    static const int value = 0;
  };

  template<class It1, class It2, int Size>
  struct equal_to<It1, It2, Size, Size> {
    static const int value = 1;
  };
};

typedef cons<int, cons<char, nil> > two_context;
typedef iterator<two_context> begin_iter;
typedef iterator<nil> end_iter;

static_assert(begin_iter::defaults<begin_iter, end_iter>::first == 2, "");
static_assert(begin_iter::defaults<begin_iter, end_iter>::second == 0, "");
static_assert(begin_iter::equal_to<begin_iter, end_iter>::value == 0, "");

int main()
{
  return begin_iter::defaults<begin_iter, end_iter>::second;
}
