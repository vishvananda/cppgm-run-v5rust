// Direct reference binding through a conversion function beats constructing a
// temporary, even when the constructor has a less cv-qualified source binding.

int selected_constructor = 0;

struct target;

struct source
{
  operator const target&() const;
};

struct target
{
  target() {}
  target(source&) { selected_constructor = 1; }
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
  return selected_constructor;
}
