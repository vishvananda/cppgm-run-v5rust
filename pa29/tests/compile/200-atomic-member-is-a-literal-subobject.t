// N3485 3.9/10 disqualifies a class with a volatile non-static data member
// from being a literal type, so it cannot have a constexpr constructor.
// _Atomic is a separate extension and carries no such rule: libc++'s
// atomic_flag holds an _Atomic member and declares a constexpr constructor,
// and clang accepts it.  The layout fact that answers "may this be zero
// initialized wholesale" conflates the two on purpose, so the literal-type
// rule has to ask the narrower question.

struct atomic_holder
{
  _Atomic(bool) flag_;
  constexpr explicit atomic_holder(bool value) : flag_(value) {}
};

// Through a base and a member, since the check walks subobjects.
struct derived_holder : atomic_holder
{
  atomic_holder nested_;
  constexpr derived_holder(bool value)
    : atomic_holder(value), nested_(value) {}
};

// A volatile member still disqualifies the class, so the narrower question is
// still being asked rather than skipped.
struct volatile_holder
{
  volatile bool flag_;
  volatile_holder(bool value) : flag_(value) {}
};

int main()
{
  atomic_holder made(true);
  derived_holder nested_made(false);
  volatile_holder plain(true);
  int status = 0;
  if (sizeof(made) == 0) status = 1;
  if (sizeof(nested_made) < sizeof(made)) status = 2;
  if (sizeof(plain) == 0) status = 3;
  return status;
}
