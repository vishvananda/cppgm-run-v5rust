struct NonTrivialAssign {
  NonTrivialAssign & operator=(const NonTrivialAssign &);
};

struct TrivialAssign {
  TrivialAssign & operator=(const TrivialAssign &) = default;
};

struct Plain {
  unsigned long value;
};

static_assert(__is_trivially_assignable(unsigned long &, const unsigned long &), "scalar assign");
static_assert(!__is_trivially_assignable(const unsigned long &, const unsigned long &),
              "const scalar assign");
static_assert(__is_trivially_assignable(Plain &, const Plain &), "plain assign");
static_assert(__is_trivially_assignable(TrivialAssign &, const TrivialAssign &), "copy assign");
static_assert(!__is_trivially_assignable(NonTrivialAssign &, const NonTrivialAssign &),
              "non-trivial copy assign");

int main() { return 0; }
