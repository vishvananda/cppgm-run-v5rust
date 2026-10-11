#if !defined __is_identifier
#error "__is_identifier should be visible to defined"
#endif

#define HAS_TRAIT(T) (__has_extension(T) || !__is_identifier(__##T))

#if !HAS_TRAIT(is_constructible)
#error "missing constructible trait probe"
#endif

#if !HAS_TRAIT(is_nothrow_constructible)
#error "missing nothrow constructible trait probe"
#endif

#if __is_identifier(__is_constructible)
#error "__is_constructible should be a recognized trait builtin"
#endif

#if __is_identifier(__is_nothrow_constructible)
#error "__is_nothrow_constructible should be a recognized trait builtin"
#endif

#if !__is_identifier(ordinary_identifier_for_probe)
#error "ordinary names should remain identifiers"
#endif

#if __is_identifier(static_assert)
#error "language keywords should not be identifiers"
#endif

void may_throw() {}

struct Base
{
  Base() {}
  explicit Base(const Base&) { may_throw(); }
};

struct NoexCopy : Base
{
  NoexCopy() {}
  NoexCopy(const NoexCopy& rhs) noexcept : Base(rhs) {}
};

static_assert(__is_constructible(NoexCopy, const NoexCopy&), "");
static_assert(__is_nothrow_constructible(NoexCopy, const NoexCopy&), "");

int main() { return 0; }
