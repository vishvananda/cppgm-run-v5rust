// A variable template specialization is instantiated once and memoized, so
// whatever its initializer folded to is what every later reader sees.  Naming
// one in an operand a disjunction has already short-circuited past must not
// cache it without a value: the specialization's value belongs to the
// specialization, not to whichever context happened to name it first.  The
// reads below are independent of the short-circuiting one and each requires a
// constant, so a devalued cache entry shows up as a non-constant expression.

template<class T>
const bool small_v = sizeof(T) <= 4;

template<bool B>
struct pick
{
  static const int value = 1;
};

template<>
struct pick<false>
{
  static const int value = 2;
};

template<class T>
struct warm
{
  // True on the left, so the right operand is never evaluated -- but naming
  // it still instantiates small_v<T>.
  static const bool ignored = true || small_v<T>;
};

// The same shape in a noexcept-specifier, which is where libc++ meets it:
// __swap_allocator's own specification reads a trait a caller already
// short-circuited past.
template<class T>
void later() noexcept(small_v<T>)
{
}

int main()
{
  // Plain locals, not const ones: a const bool local folds into the enclosing
  // condition and that shape is not what this test is about.
  bool warmed_small = warm<char>::ignored;
  bool warmed_large = warm<double>::ignored;

  later<char>();
  later<double>();

  int small_answer = pick<small_v<char> >::value;
  int large_answer = pick<small_v<double> >::value;

  return (warmed_small && warmed_large &&
    small_answer == 1 && large_answer == 2) ? 0 : 1;
}
