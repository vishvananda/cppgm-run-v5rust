struct Plain
{
  int value;
};

struct Nested
{
  Plain value;
};

struct DefaultedCopy
{
  DefaultedCopy() = default;
  DefaultedCopy(const DefaultedCopy&) = default;
  int value;
};

struct NonTrivialCopy
{
  NonTrivialCopy(const NonTrivialCopy&);
  int value;
};

struct NonTrivialDestructor
{
  ~NonTrivialDestructor();
  int value;
};

static_assert(__is_trivially_copyable(Plain), "plain class");
static_assert(__is_trivially_copyable(Nested), "nested class");
static_assert(__is_trivially_copyable(DefaultedCopy), "defaulted copy");
static_assert(!__is_trivially_copyable(NonTrivialCopy), "non-trivial copy");
static_assert(!__is_trivially_copyable(NonTrivialDestructor),
              "non-trivial destructor");

int main()
{
  return 0;
}
