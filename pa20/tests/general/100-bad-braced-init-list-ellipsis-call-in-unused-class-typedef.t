// VALIDATION: compile-fail
// A braced-init-list cannot be passed through an ellipsis, and an invalid
// typedef in a non-template class must be diagnosed even when it is unused.

struct Probe
{
  static int helper(int&);
  static char helper(...);

  typedef decltype(helper({})) type;
};
