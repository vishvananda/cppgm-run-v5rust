// VALIDATION: compile-pass
// N3485 focus: 10.3 [class.virtual]
// A pure-specifier is valid on an overriding virtual declaration even when
// the declaration does not repeat the `virtual` keyword.

struct base
{
  virtual char const * what() const = 0;
};

struct mid : base
{
  char const * what() const override = 0;
};

struct leaf : mid
{
  char const * what() const override
  {
    return "ok";
  }
};

int main()
{
  leaf object;
  return object.what()[0] == 'o' ? 0 : 1;
}
