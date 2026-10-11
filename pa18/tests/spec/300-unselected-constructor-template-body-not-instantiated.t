// VALIDATION: compile-pass
// N3485 focus: 8.5.3 [dcl.init.ref], 14.7.1 [temp.inst]

struct target;

struct source
{
  operator const target&() const;
};

struct target
{
  target() {}

  template<class T>
  target(T& value) { ++value; }
};

target stored_target;

source::operator const target&() const
{
  return stored_target;
}

void consume(const target&) {}

int main()
{
  source value;
  consume(value);
}
