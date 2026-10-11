// VALIDATION: compile-pass

struct ordinary { ordinary& operator=(ordinary&&) noexcept; };
struct lvalue_only { lvalue_only& operator=(lvalue_only&&) & noexcept; };
struct rvalue_only { rvalue_only& operator=(rvalue_only&&) && noexcept; };

struct deleted_exact
{
  deleted_exact(int) noexcept;
  deleted_exact& operator=(int) = delete;
};

static_assert(__is_nothrow_assignable(ordinary, ordinary),
              "an unqualified assignment member accepts a class xvalue");
static_assert(!__is_assignable(lvalue_only, lvalue_only),
              "an lvalue-qualified member rejects a class xvalue");
static_assert(__is_assignable(lvalue_only&, lvalue_only),
              "an lvalue-qualified member accepts a class lvalue");
static_assert(__is_nothrow_assignable(rvalue_only, rvalue_only),
              "an rvalue-qualified member accepts a class xvalue");
static_assert(!__is_assignable(rvalue_only&, rvalue_only),
              "an rvalue-qualified member rejects a class lvalue");
static_assert(!__is_assignable(deleted_exact&, int),
              "a deleted exact match beats a converting assignment");
static_assert(!__is_nothrow_assignable(deleted_exact&, int),
              "a deleted exact match is not nothrow assignable");
